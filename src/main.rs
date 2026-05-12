use std::collections::VecDeque;
use std::env;
use std::path::PathBuf;

use action_compiler_vm::{
    ACTION_OS_PRESET, AddressRange, BusAccess, BusEvent, CpuError, CpuRegisters, CpuStep,
    ImageKind, VmConfig,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "inspect" => inspect(parse_options(args.collect())?.config),
        "run" => run_vm(parse_options(args.collect())?),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn inspect(config: VmConfig) -> Result<(), String> {
    let vm = config.load()?;
    println!("loaded {} image(s)", vm.images().len());
    for image in vm.images() {
        println!(
            "{:?}: {} byte(s), ${:04X}-${:04X}, checksum16=${:04X}, crc32=${:08X}",
            image.kind,
            image.metadata.size,
            image.metadata.base,
            image.metadata.end,
            image.metadata.checksum16,
            image.metadata.crc32
        );
        if let Some(header) = image.car_header {
            println!(
                "  CAR type=${:08X}, header checksum=${:08X}",
                header.cartridge_type, header.checksum
            );
        }
        if let Some(mapping) = image.cartridge_mapping {
            println!(
                "  cart window ${:04X}-${:04X}, bank_size={}, banks={}, active_bank={}",
                mapping.window_start,
                mapping.window_end,
                mapping.bank_size,
                mapping.bank_count,
                mapping.active_bank
            );
        }
    }
    Ok(())
}

fn run_vm(options: CliOptions) -> Result<(), String> {
    let config = options.config.clone();
    config.validate_for_execution()?;
    let mut vm = config.load()?;
    for watchpoint in &options.watchpoints {
        vm.bus_mut().add_watchpoint(*watchpoint);
    }
    for watch_range in &options.watch_ranges {
        vm.bus_mut().add_watch_range(*watch_range);
    }
    vm.reset_cpu();
    println!(
        "compiler VM loaded {} image(s); reset PC=${:04X}",
        vm.images().len(),
        vm.cpu().registers().pc
    );

    let mut history = StepHistory::new(options.history_len);
    for step_index in 0..options.max_steps {
        match vm.step_cpu() {
            Ok(step) => {
                if options.should_trace(step.pc) {
                    print_step(&step);
                }
                let reached_trace_until = options.trace_until == Some(step.pc);
                history.push(step);
                if reached_trace_until {
                    print_stop_report(
                        "trace-until reached",
                        Some(step.registers_after),
                        Some(&history),
                        vm.bus().events(),
                        vm.bus().cartridge().map(|cart| cart.mapping_info()),
                    );
                    return Ok(());
                }
            }
            Err(CpuError::UnsupportedOpcode { pc, opcode }) => {
                print_stop_report(
                    &format!("unsupported opcode ${opcode:02X} at ${pc:04X}"),
                    Some(vm.cpu().registers()),
                    Some(&history),
                    vm.bus().events(),
                    vm.bus().cartridge().map(|cart| cart.mapping_info()),
                );
                return Err(format!("unsupported opcode ${opcode:02X} at ${pc:04X}"));
            }
            Err(CpuError::Halted) => {
                print_stop_report(
                    "CPU halted",
                    Some(vm.cpu().registers()),
                    Some(&history),
                    vm.bus().events(),
                    vm.bus().cartridge().map(|cart| cart.mapping_info()),
                );
                return Err("CPU halted".to_string());
            }
        }

        if step_index + 1 == options.max_steps {
            print_stop_report(
                "max steps reached",
                Some(vm.cpu().registers()),
                Some(&history),
                vm.bus().events(),
                vm.bus().cartridge().map(|cart| cart.mapping_info()),
            );
        }
    }

    println!(
        "stopped after {} step(s), cycles={}, PC=${:04X}",
        options.max_steps,
        vm.cpu().cycles(),
        vm.cpu().registers().pc
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    config: VmConfig,
    max_steps: u64,
    trace_pc: bool,
    trace_ranges: Vec<AddressRange>,
    trace_until: Option<u16>,
    history_len: usize,
    watchpoints: Vec<u16>,
    watch_ranges: Vec<AddressRange>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            config: VmConfig::default(),
            max_steps: 1_000,
            trace_pc: false,
            trace_ranges: Vec::new(),
            trace_until: None,
            history_len: 64,
            watchpoints: Vec::new(),
            watch_ranges: Vec::new(),
        }
    }
}

impl CliOptions {
    fn should_trace(&self, pc: u16) -> bool {
        self.trace_pc || self.trace_ranges.iter().any(|range| range.contains(pc))
    }
}

fn parse_options(args: Vec<String>) -> Result<CliOptions, String> {
    let mut config = VmConfig::default();
    let mut max_steps = 1_000;
    let mut trace_pc = false;
    let mut trace_ranges = Vec::new();
    let mut trace_until = None;
    let mut history_len = 64;
    let mut watchpoints = Vec::new();
    let mut watch_ranges = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--max-cycles" | "--max-steps" => {
                index += 1;
                let value = required_value(&args, index, "--max-cycles")?;
                max_steps = value
                    .parse()
                    .map_err(|_| format!("invalid max step count `{value}`"))?;
            }
            "--trace-pc" => {
                trace_pc = true;
            }
            "--trace-range" => {
                index += 1;
                let value = required_value(&args, index, "--trace-range")?;
                trace_ranges.push(parse_range(value)?);
            }
            "--trace-until" => {
                index += 1;
                let value = required_value(&args, index, "--trace-until")?;
                trace_until = Some(parse_address(value)?);
            }
            "--history" => {
                index += 1;
                let value = required_value(&args, index, "--history")?;
                history_len = value
                    .parse()
                    .map_err(|_| format!("invalid history length `{value}`"))?;
            }
            "--watch" => {
                index += 1;
                let address = required_value(&args, index, "--watch").and_then(parse_address)?;
                watchpoints.push(address);
            }
            "--watch-range" => {
                index += 1;
                let value = required_value(&args, index, "--watch-range")?;
                watch_ranges.push(parse_range(value)?);
            }
            "--preset" => {
                index += 1;
                let value = required_value(&args, index, "--preset")?;
                apply_preset(&mut config, value)?;
            }
            "--cart" => {
                index += 1;
                let path = required_value(&args, index, "--cart")?;
                config.cartridge = Some(PathBuf::from(path));
            }
            "--cart-base" => {
                index += 1;
                let value = required_value(&args, index, "--cart-base")?;
                config.cartridge_base = parse_address(value)?;
            }
            "--os" => {
                index += 1;
                let path = required_value(&args, index, "--os")?;
                config.os_rom = Some(PathBuf::from(path));
            }
            "--os-base" => {
                index += 1;
                let value = required_value(&args, index, "--os-base")?;
                config.os_base = parse_address(value)?;
            }
            "--source" => {
                index += 1;
                let path = required_value(&args, index, "--source")?;
                config.source = Some(PathBuf::from(path));
            }
            "--map" => {
                index += 1;
                let value = required_value(&args, index, "--map")?;
                config.extra_images.push(parse_image_map(value)?);
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(CliOptions {
        config,
        max_steps,
        trace_pc,
        trace_ranges,
        trace_until,
        history_len,
        watchpoints,
        watch_ranges,
    })
}

fn apply_preset(config: &mut VmConfig, value: &str) -> Result<(), String> {
    match value {
        "action-os" => {
            config.apply_preset(ACTION_OS_PRESET);
            Ok(())
        }
        other => Err(format!("unknown preset `{other}`")),
    }
}

fn required_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_image_map(value: &str) -> Result<(ImageKind, PathBuf, u16), String> {
    let mut parts = value.splitn(3, ':');
    let kind = match parts.next() {
        Some("ram") => ImageKind::Ram,
        Some("rom") => ImageKind::Rom,
        Some("cart") => ImageKind::Cartridge,
        Some(other) => return Err(format!("unknown image kind `{other}`")),
        None => return Err("missing image kind".to_string()),
    };
    let path = parts
        .next()
        .ok_or_else(|| "image map must be kind:path:address".to_string())?;
    let base = parts
        .next()
        .ok_or_else(|| "image map must be kind:path:address".to_string())
        .and_then(parse_address)?;

    Ok((kind, PathBuf::from(path), base))
}

fn parse_address(value: &str) -> Result<u16, String> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed.strip_prefix('$') {
        u16::from_str_radix(hex, 16)
    } else if let Some(hex) = trimmed.strip_prefix("0x") {
        u16::from_str_radix(hex, 16)
    } else {
        trimmed.parse()
    };

    parsed.map_err(|_| format!("invalid address `{value}`"))
}

fn parse_range(value: &str) -> Result<AddressRange, String> {
    let Some((start, end)) = value.split_once(':') else {
        return Err(format!("range `{value}` must be start:end"));
    };
    let start = parse_address(start)?;
    let end = parse_address(end)?;
    if start > end {
        return Err(format!("range `{value}` starts after it ends"));
    }
    Ok(AddressRange { start, end })
}

#[derive(Debug)]
struct StepHistory {
    limit: usize,
    steps: VecDeque<CpuStep>,
}

impl StepHistory {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            steps: VecDeque::new(),
        }
    }

    fn push(&mut self, step: CpuStep) {
        if self.limit == 0 {
            return;
        }
        if self.steps.len() == self.limit {
            self.steps.pop_front();
        }
        self.steps.push_back(step);
    }

    fn steps(&self) -> impl Iterator<Item = &CpuStep> {
        self.steps.iter()
    }
}

fn print_step(step: &CpuStep) {
    let regs = step.registers_before;
    println!(
        "{:08} PC=${:04X} OP=${:02X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X} P=${:02X}",
        step.cycles, step.pc, step.opcode, regs.a, regs.x, regs.y, regs.sp, regs.status
    );
}

fn print_stop_report(
    reason: &str,
    registers: Option<CpuRegisters>,
    history: Option<&StepHistory>,
    events: &[BusEvent],
    cartridge: Option<action_compiler_vm::CartridgeMappingInfo>,
) {
    eprintln!("stop: {reason}");
    if let Some(regs) = registers {
        eprintln!(
            "regs: PC=${:04X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X} P=${:02X}",
            regs.pc, regs.a, regs.x, regs.y, regs.sp, regs.status
        );
    }
    if let Some(cartridge) = cartridge {
        eprintln!(
            "cart: window=${:04X}-${:04X} bank={}/{} bank_size={}",
            cartridge.window_start,
            cartridge.window_end,
            cartridge.active_bank,
            cartridge.bank_count,
            cartridge.bank_size
        );
    }
    if let Some(history) = history {
        eprintln!("recent instructions:");
        for step in history.steps() {
            eprint!("  ");
            print_step_stderr(step);
        }
    }
    if !events.is_empty() {
        eprintln!("bus events:");
        for event in events
            .iter()
            .rev()
            .take(64)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            let access = match event.access {
                BusAccess::Read => "R",
                BusAccess::Write => "W",
            };
            eprintln!(
                "  {access} ${:04X}=${:02X} {:?}",
                event.address, event.value, event.region
            );
        }
    }
}

fn print_step_stderr(step: &CpuStep) {
    let regs = step.registers_before;
    eprintln!(
        "{:08} PC=${:04X} OP=${:02X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X} P=${:02X}",
        step.cycles, step.pc, step.opcode, regs.a, regs.x, regs.y, regs.sp, regs.status
    );
}

fn print_help() {
    println!(
        "action-compiler-vm\n\n\
         Usage:\n  \
         action-compiler-vm inspect [options]\n  \
         action-compiler-vm run [options]\n\n\
         Options:\n  \
         --preset <name>      Mapping preset, currently action-os\n  \
         --cart <path>        Load an Action! cartridge image\n  \
         --cart-base <addr>   Cartridge base address, default $A000\n  \
         --os <path>          Load an Atari OS ROM image at $C000\n  \
         --os-base <addr>     OS ROM base address, default $C000\n  \
         --max-cycles <n>     Run at most n CPU steps, default 1000\n  \
         --trace-pc           Print one line per executed instruction\n  \
         --trace-range <a:b>  Print instructions with PC inside the range\n  \
         --trace-until <addr> Stop after executing an instruction at addr\n  \
         --history <n>        Recent instruction count in stop reports, default 64\n  \
         --watch <addr>       Record bus reads/writes at addr\n  \
         --watch-range <a:b>  Record bus reads/writes inside the range\n  \
         --source <path>      Source file reserved for the future compiler harness\n  \
         --map <k:p:a>        Map an extra image: ram:path:addr, rom:path:addr, cart:path:addr"
    );
}
