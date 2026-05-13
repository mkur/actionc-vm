use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::PathBuf;

use action_compiler_vm::{
    ACTION_MONITOR_KEY_CODE, ACTION_OS_PRESET, ATARI_KEY_C, ATARI_KEY_E, ATARI_KEY_RETURN,
    ActionEditorLine, ActionSourceInjectionReport, AddressRange, BusAccess, BusEvent, CpuError,
    CpuRegisters, CpuStep, Hotpatch, ImageKind, TextScreenSnapshot, VmConfig,
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
    if let Some(cartridge) = vm.bus().cartridge() {
        println!(
            "visible cart vectors: INIT=${:04X} START=${:04X} FLAGS=${:04X}",
            cart_word(cartridge, 0xBFFA),
            cart_word(cartridge, 0xBFFC),
            cart_word(cartridge, 0xBFFE)
        );
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
    for key_code in &options.key_codes {
        vm.bus_mut().queue_key_code(*key_code);
    }
    for bytes in &options.scripted_cio_inputs {
        vm.bus_mut().queue_scripted_cio_input_bytes(bytes);
    }
    vm.reset_cpu();
    let mut deferred_key_codes = options.deferred_key_codes.clone();
    let mut deferred_scripted_cio_inputs = options.deferred_scripted_cio_inputs.clone();
    let mut deferred_source_injections = options.deferred_source_injections.clone();
    let mut editor_line_dump_pcs = options.editor_line_dump_pcs.clone();
    let mut screen_dump_pcs = options.screen_dump_pcs.clone();
    println!(
        "compiler VM loaded {} image(s); reset PC=${:04X}",
        vm.images().len(),
        vm.cpu().registers().pc
    );

    let mut history = StepHistory::new(options.history_len);
    for step_index in 0..options.max_steps {
        let pc = vm.cpu().registers().pc;
        let mut source_index = 0;
        while source_index < deferred_source_injections.len() {
            if deferred_source_injections[source_index].pc == pc {
                let deferred = deferred_source_injections.remove(source_index);
                let source = fs::read(&deferred.path).map_err(|err| {
                    format!(
                        "failed to read source `{}` for injection: {err}",
                        deferred.path.display()
                    )
                })?;
                let report = vm.bus_mut().inject_action_source(&source)?;
                print_source_injection_report(&deferred, &report);
                print_editor_lines(vm.bus())?;
            } else {
                source_index += 1;
            }
        }

        let mut dump_index = 0;
        while dump_index < editor_line_dump_pcs.len() {
            if editor_line_dump_pcs[dump_index] == pc {
                let dump_pc = editor_line_dump_pcs.remove(dump_index);
                eprintln!("Action! editor lines at PC=${dump_pc:04X}:");
                print_editor_lines(vm.bus())?;
            } else {
                dump_index += 1;
            }
        }

        let mut screen_dump_index = 0;
        while screen_dump_index < screen_dump_pcs.len() {
            if screen_dump_pcs[screen_dump_index] == pc {
                let dump_pc = screen_dump_pcs.remove(screen_dump_index);
                eprintln!("text screen at PC=${dump_pc:04X}:");
                print_text_screen(&vm.bus().text_screen_snapshot(40, 24));
            } else {
                screen_dump_index += 1;
            }
        }

        let mut deferred_index = 0;
        while deferred_index < deferred_scripted_cio_inputs.len() {
            if deferred_scripted_cio_inputs[deferred_index].after_pc == Some(pc) {
                deferred_scripted_cio_inputs[deferred_index].after_pc = None;
            }
            if deferred_scripted_cio_inputs[deferred_index].after_pc.is_none()
                && deferred_scripted_cio_inputs[deferred_index].pc == pc
            {
                let deferred = deferred_scripted_cio_inputs.remove(deferred_index);
                vm.bus_mut().queue_scripted_cio_input_bytes(&deferred.bytes);
                eprintln!(
                    "queued {} scripted CIO byte(s) at PC=${:04X}",
                    deferred.bytes.len(),
                    deferred.pc
                );
            } else {
                deferred_index += 1;
            }
        }

        let mut deferred_index = 0;
        while deferred_index < deferred_key_codes.len() {
            if deferred_key_codes[deferred_index].after_pc == Some(pc) {
                deferred_key_codes[deferred_index].after_pc = None;
            }
            if deferred_key_codes[deferred_index].after_pc.is_none()
                && deferred_key_codes[deferred_index].pc == pc
            {
                let deferred = deferred_key_codes.remove(deferred_index);
                vm.bus_mut().queue_key_code(deferred.key_code);
                eprintln!(
                    "queued key ${:02X} at PC=${:04X}",
                    deferred.key_code, deferred.pc
                );
            } else {
                deferred_index += 1;
            }
        }
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
                    print_run_observations(
                        vm.bus(),
                        options.dump_screen_on_stop,
                        &options.memory_dump_ranges,
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
                print_run_observations(
                    vm.bus(),
                    options.dump_screen_on_stop,
                    &options.memory_dump_ranges,
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
                print_run_observations(
                    vm.bus(),
                    options.dump_screen_on_stop,
                    &options.memory_dump_ranges,
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
            print_run_observations(
                vm.bus(),
                options.dump_screen_on_stop,
                &options.memory_dump_ranges,
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
    key_codes: Vec<u8>,
    scripted_cio_inputs: Vec<Vec<u8>>,
    deferred_scripted_cio_inputs: Vec<DeferredScriptedCioInput>,
    deferred_key_codes: Vec<DeferredKeyCode>,
    deferred_source_injections: Vec<DeferredSourceInjection>,
    editor_line_dump_pcs: Vec<u16>,
    screen_dump_pcs: Vec<u16>,
    memory_dump_ranges: Vec<AddressRange>,
    dump_screen_on_stop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredKeyCode {
    pc: u16,
    key_code: u8,
    after_pc: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredScriptedCioInput {
    pc: u16,
    bytes: Vec<u8>,
    after_pc: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredSourceInjection {
    pc: u16,
    path: PathBuf,
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
            key_codes: Vec::new(),
            scripted_cio_inputs: Vec::new(),
            deferred_scripted_cio_inputs: Vec::new(),
            deferred_key_codes: Vec::new(),
            deferred_source_injections: Vec::new(),
            editor_line_dump_pcs: Vec::new(),
            screen_dump_pcs: Vec::new(),
            memory_dump_ranges: Vec::new(),
            dump_screen_on_stop: false,
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
    let mut key_codes = Vec::new();
    let mut scripted_cio_inputs = Vec::new();
    let mut deferred_scripted_cio_inputs = Vec::new();
    let mut deferred_key_codes = Vec::new();
    let mut deferred_source_injections = Vec::new();
    let mut editor_line_dump_pcs = Vec::new();
    let mut screen_dump_pcs = Vec::new();
    let mut memory_dump_ranges = Vec::new();
    let mut dump_screen_on_stop = false;
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
            "--key-code" => {
                index += 1;
                let value = required_value(&args, index, "--key-code")?;
                key_codes.push(parse_byte(value)?);
            }
            "--q-input" => {
                index += 1;
                let value = required_value(&args, index, "--q-input")?;
                scripted_cio_inputs.push(parse_scripted_cio_input(value));
            }
            "--q-input-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--q-input-at-pc")?;
                deferred_scripted_cio_inputs.push(parse_scripted_cio_input_at_pc(value)?);
            }
            "--q-input-at-pc-after" => {
                index += 1;
                let value = required_value(&args, index, "--q-input-at-pc-after")?;
                deferred_scripted_cio_inputs.push(parse_scripted_cio_input_at_pc_after(value)?);
            }
            "--trace-cio" => {
                config.trace_cio = true;
            }
            "--key-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--key-at-pc")?;
                deferred_key_codes.push(parse_key_at_pc(value)?);
            }
            "--key-at-pc-after" => {
                index += 1;
                let value = required_value(&args, index, "--key-at-pc-after")?;
                deferred_key_codes.push(parse_key_at_pc_after(value)?);
            }
            "--monitor-key" => {
                key_codes.push(ACTION_MONITOR_KEY_CODE);
            }
            "--monitor-key-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--monitor-key-at-pc")?;
                deferred_key_codes.push(DeferredKeyCode {
                    pc: parse_address(value)?,
                    key_code: ACTION_MONITOR_KEY_CODE,
                    after_pc: None,
                });
            }
            "--action-command-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--action-command-at-pc")?;
                deferred_key_codes.extend(parse_action_command_at_pc(value)?);
            }
            "--inject-source-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--inject-source-at-pc")?;
                deferred_source_injections.push(parse_source_injection_at_pc(value)?);
            }
            "--dump-editor-lines-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--dump-editor-lines-at-pc")?;
                editor_line_dump_pcs.push(parse_address(value)?);
            }
            "--dump-screen-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--dump-screen-at-pc")?;
                screen_dump_pcs.push(parse_address(value)?);
            }
            "--dump-screen-on-stop" => {
                dump_screen_on_stop = true;
            }
            "--dump-range-on-stop" => {
                index += 1;
                let value = required_value(&args, index, "--dump-range-on-stop")?;
                memory_dump_ranges.push(parse_range(value)?);
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
            "--host-file" => {
                index += 1;
                let value = required_value(&args, index, "--host-file")?;
                config.host_files.push(parse_host_file_map(value)?);
            }
            "--map" => {
                index += 1;
                let value = required_value(&args, index, "--map")?;
                config.extra_images.push(parse_image_map(value)?);
            }
            "--hotpatch" => {
                index += 1;
                let value = required_value(&args, index, "--hotpatch")?;
                config.hotpatches.push(parse_hotpatch(value)?);
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
        key_codes,
        scripted_cio_inputs,
        deferred_scripted_cio_inputs,
        deferred_key_codes,
        deferred_source_injections,
        editor_line_dump_pcs,
        screen_dump_pcs,
        memory_dump_ranges,
        dump_screen_on_stop,
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

fn parse_hotpatch(value: &str) -> Result<Hotpatch, String> {
    match value {
        "action-q-input" => Ok(Hotpatch::ActionQueuedInput),
        "action-headless-getkey" => Ok(Hotpatch::ActionHeadlessGetkey),
        other => Err(format!("unknown hotpatch `{other}`")),
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

fn parse_byte(value: &str) -> Result<u8, String> {
    if let Some(key_code) = parse_named_key(value) {
        return Ok(key_code);
    }
    let parsed = parse_address(value)?;
    u8::try_from(parsed).map_err(|_| format!("byte value `{value}` is outside $00-$FF"))
}

fn parse_named_key(value: &str) -> Option<u8> {
    match value.trim().to_ascii_lowercase().as_str() {
        "return" | "ret" | "enter" => Some(ATARI_KEY_RETURN),
        "c" => Some(ATARI_KEY_C),
        "e" => Some(ATARI_KEY_E),
        "monitor" => Some(ACTION_MONITOR_KEY_CODE),
        _ => None,
    }
}

fn parse_scripted_cio_input(value: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            bytes.push(ascii_to_atascii(ch));
            continue;
        }

        match chars.next() {
            Some('n') | Some('r') => bytes.push(0x9B),
            Some('\\') => bytes.push(b'\\'),
            Some(other) => {
                bytes.push(b'\\');
                bytes.push(ascii_to_atascii(other));
            }
            None => bytes.push(b'\\'),
        }
    }
    bytes
}

fn parse_scripted_cio_input_at_pc(value: &str) -> Result<DeferredScriptedCioInput, String> {
    let Some((pc, text)) = value.split_once(':') else {
        return Err(format!("scripted CIO trigger `{value}` must be pc:text"));
    };
    Ok(DeferredScriptedCioInput {
        pc: parse_address(pc)?,
        bytes: parse_scripted_cio_input(text),
        after_pc: None,
    })
}

fn parse_scripted_cio_input_at_pc_after(value: &str) -> Result<DeferredScriptedCioInput, String> {
    let parts = value.splitn(3, ':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "gated scripted CIO trigger `{value}` must be after_pc:pc:text"
        ));
    }
    Ok(DeferredScriptedCioInput {
        after_pc: Some(parse_address(parts[0])?),
        pc: parse_address(parts[1])?,
        bytes: parse_scripted_cio_input(parts[2]),
    })
}

fn parse_host_file_map(value: &str) -> Result<(String, PathBuf), String> {
    let Some((name, path)) = value.split_once(':') else {
        return Err(format!("host file `{value}` must be name:path"));
    };
    if name.trim().is_empty() {
        return Err("host file name must not be empty".to_string());
    }
    if path.is_empty() {
        return Err("host file path must not be empty".to_string());
    }
    Ok((name.to_string(), PathBuf::from(path)))
}

fn ascii_to_atascii(ch: char) -> u8 {
    if ch == '\n' || ch == '\r' {
        0x9B
    } else if ch.is_ascii() {
        ch as u8
    } else {
        b'?'
    }
}

fn parse_key_at_pc(value: &str) -> Result<DeferredKeyCode, String> {
    let Some((pc, key_code)) = value.split_once(':') else {
        return Err(format!("key trigger `{value}` must be pc:key"));
    };
    Ok(DeferredKeyCode {
        pc: parse_address(pc)?,
        key_code: parse_byte(key_code)?,
        after_pc: None,
    })
}

fn parse_key_at_pc_after(value: &str) -> Result<DeferredKeyCode, String> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "gated key trigger `{value}` must be after_pc:pc:key"
        ));
    }
    Ok(DeferredKeyCode {
        after_pc: Some(parse_address(parts[0])?),
        pc: parse_address(parts[1])?,
        key_code: parse_byte(parts[2])?,
    })
}

fn parse_action_command_at_pc(value: &str) -> Result<Vec<DeferredKeyCode>, String> {
    let Some((pc, command)) = value.split_once(':') else {
        return Err(format!(
            "Action! command trigger `{value}` must be pc:command"
        ));
    };
    let pc = parse_address(pc)?;
    let key_code = match command.trim().to_ascii_lowercase().as_str() {
        "compile" | "c" => ATARI_KEY_C,
        "editor" | "edit" | "e" => ATARI_KEY_E,
        other => return Err(format!("unknown Action! monitor command `{other}`")),
    };
    Ok(vec![
        DeferredKeyCode {
            pc,
            key_code,
            after_pc: None,
        },
        DeferredKeyCode {
            pc,
            key_code: ATARI_KEY_RETURN,
            after_pc: None,
        },
    ])
}

fn parse_source_injection_at_pc(value: &str) -> Result<DeferredSourceInjection, String> {
    let Some((pc, path)) = value.split_once(':') else {
        return Err(format!("source injection `{value}` must be pc:path"));
    };
    if path.is_empty() {
        return Err("source injection path must not be empty".to_string());
    }
    Ok(DeferredSourceInjection {
        pc: parse_address(pc)?,
        path: PathBuf::from(path),
    })
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

fn print_source_injection_report(
    deferred: &DeferredSourceInjection,
    report: &ActionSourceInjectionReport,
) {
    eprintln!(
        "injected source `{}` at PC=${:04X}: {} line(s), first={}, last={}, allocated={} byte(s), free_head=${:04X}",
        deferred.path.display(),
        deferred.pc,
        report.line_count,
        format_optional_address(report.first_line),
        format_optional_address(report.last_line),
        report.allocated_bytes,
        report.free_head
    );
}

fn print_editor_lines(bus: &action_compiler_vm::Bus) -> Result<(), String> {
    let lines = bus.action_editor_lines()?;
    if lines.is_empty() {
        eprintln!("  <empty>");
        return Ok(());
    }
    for (index, line) in lines.iter().enumerate() {
        print_editor_line(index, line);
    }
    Ok(())
}

fn print_editor_line(index: usize, line: &ActionEditorLine) {
    eprintln!(
        "  {:03} @ ${:04X}: prev=${:04X} next=${:04X} alloc={} len={} `{}`",
        index + 1,
        line.address,
        line.previous,
        line.next,
        line.allocation_size,
        line.length,
        String::from_utf8_lossy(&line.text)
    );
}

fn print_run_observations(
    bus: &action_compiler_vm::Bus,
    dump_screen: bool,
    memory_dump_ranges: &[AddressRange],
) {
    if bus.speaker_write_count() != 0 {
        let last = bus
            .last_speaker_write()
            .map(|value| format!("${value:02X}"))
            .unwrap_or_else(|| "<none>".to_string());
        eprintln!("speaker writes: {} last={last}", bus.speaker_write_count());
    }
    if !bus.cio_channel0_output().is_empty() {
        eprintln!("CIO E: channel 0 output:");
        for line in bus.decoded_cio_channel0_output().lines() {
            eprintln!("  {line}");
        }
    }
    if let Some(error_line) = bus.visible_action_error() {
        eprintln!("visible Action! error: `{error_line}`");
    }
    if dump_screen {
        print_text_screen(&bus.text_screen_snapshot(40, 24));
    }
    print_memory_dumps(bus, memory_dump_ranges);
}

fn print_memory_dumps(bus: &action_compiler_vm::Bus, ranges: &[AddressRange]) {
    for range in ranges {
        eprintln!("memory ${:04X}-${:04X}:", range.start, range.end);
        let mut line_start = range.start;
        while line_start <= range.end {
            let line_end = line_start.saturating_add(15).min(range.end);
            let mut hex = String::new();
            let mut text = String::new();
            let mut address = line_start;
            while address <= line_end {
                let value = bus.ram().read(address);
                hex.push_str(&format!(" {value:02X}"));
                text.push(memory_dump_char(value));
                if address == u16::MAX {
                    break;
                }
                address += 1;
            }
            eprintln!("  ${line_start:04X}:{hex:<48} |{text}|");
            if line_end == u16::MAX {
                break;
            }
            line_start = line_end + 1;
        }
    }
}

fn memory_dump_char(value: u8) -> char {
    match value {
        0x20..=0x7e => value as char,
        0x9b => '~',
        _ => '.',
    }
}

fn print_text_screen(snapshot: &TextScreenSnapshot) {
    eprintln!(
        "screen base=${:04X}, {}x{}:",
        snapshot.base, snapshot.columns, snapshot.rows
    );
    for line in &snapshot.lines {
        eprintln!("  |{}|", line.trim_end());
    }
}

fn format_optional_address(address: Option<u16>) -> String {
    address
        .map(|address| format!("${address:04X}"))
        .unwrap_or_else(|| "<none>".to_string())
}

fn cart_word(cartridge: &action_compiler_vm::Cartridge, address: u16) -> u16 {
    let lo = cartridge.read(address).unwrap_or(0xFF);
    let hi = cartridge.read(address.wrapping_add(1)).unwrap_or(0xFF);
    u16::from_le_bytes([lo, hi])
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
         --key-code <byte>    Queue one Atari keyboard code for CH ($02FC); repeatable\n  \
         --q-input <text>     Queue text for synthetic Q: CIO input; \\n becomes ATASCII EOL\n  \
         --q-input-at-pc <pc:text>\n  \
                              Queue Q: input when execution reaches pc\n  \
         --q-input-at-pc-after <after:pc:text>\n  \
                              Queue Q: input at pc, but only after after_pc was reached\n  \
         --trace-cio          Print harness CIO calls while running\n  \
         --key-at-pc <pc:k>   Queue key k when execution reaches pc\n  \
         --key-at-pc-after <after:pc:k>\n  \
                              Queue key k at pc, but only after after_pc was reached\n  \
         --monitor-key        Queue Action! Shift+Control+M ($E5)\n  \
         --monitor-key-at-pc <pc>\n  \
                              Queue Action! monitor key when execution reaches pc\n  \
         --action-command-at-pc <pc:cmd>\n  \
                              Queue monitor cmd plus Return; cmd is compile or editor\n  \
         --inject-source-at-pc <pc:path>\n  \
                              Inject host source as Action! editor lines at pc\n  \
         --dump-editor-lines-at-pc <pc>\n  \
                              Dump Action! editor line list when execution reaches pc\n  \
         --dump-screen-at-pc <pc>\n  \
                              Dump decoded 40x24 text screen when execution reaches pc\n  \
         --dump-screen-on-stop\n  \
                              Dump decoded 40x24 text screen in stop reports\n  \
         --dump-range-on-stop <a:b>\n  \
                              Dump RAM bytes in range when execution stops\n  \
         --source <path>      Source file reserved for the future compiler harness\n  \
         --host-file <n:path> Register a host file visible as H:n\n  \
         --hotpatch <name>    Apply an in-memory hotpatch, e.g. action-q-input or action-headless-getkey\n  \
         --map <k:p:a>        Map an extra image: ram:path:addr, rom:path:addr, cart:path:addr"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_key_code_option() {
        let options = parse_options(vec!["--key-code".to_string(), "$21".to_string()]).unwrap();

        assert_eq!(options.key_codes, vec![0x21]);
    }

    #[test]
    fn parses_scripted_cio_input_option() {
        let options = parse_options(vec!["--q-input".to_string(), "C\\n".to_string()]).unwrap();

        assert_eq!(options.scripted_cio_inputs, vec![vec![b'C', 0x9B]]);
    }

    #[test]
    fn parses_hotpatch_option() {
        let options = parse_options(vec![
            "--hotpatch".to_string(),
            "action-q-input".to_string(),
            "--hotpatch".to_string(),
            "action-headless-getkey".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.config.hotpatches,
            vec![Hotpatch::ActionQueuedInput, Hotpatch::ActionHeadlessGetkey]
        );
    }

    #[test]
    fn parses_repeated_key_code_options() {
        let options = parse_options(vec![
            "--key-code".to_string(),
            "$21".to_string(),
            "--key-code".to_string(),
            "$E5".to_string(),
        ])
        .unwrap();

        assert_eq!(options.key_codes, vec![0x21, 0xE5]);
    }

    #[test]
    fn parses_monitor_key_option() {
        let options = parse_options(vec!["--monitor-key".to_string()]).unwrap();

        assert_eq!(options.key_codes, vec![ACTION_MONITOR_KEY_CODE]);
    }

    #[test]
    fn parses_deferred_key_code_option() {
        let options =
            parse_options(vec!["--key-at-pc".to_string(), "$A2E0:$E5".to_string()]).unwrap();

        assert_eq!(
            options.deferred_key_codes,
            vec![DeferredKeyCode {
                pc: 0xA2E0,
                key_code: 0xE5,
                after_pc: None
            }]
        );
    }

    #[test]
    fn parses_named_key_codes() {
        let options = parse_options(vec![
            "--key-code".to_string(),
            "C".to_string(),
            "--key-code".to_string(),
            "RETURN".to_string(),
            "--key-at-pc".to_string(),
            "$B28F:E".to_string(),
        ])
        .unwrap();

        assert_eq!(options.key_codes, vec![ATARI_KEY_C, ATARI_KEY_RETURN]);
        assert_eq!(
            options.deferred_key_codes,
            vec![DeferredKeyCode {
                pc: 0xB28F,
                key_code: ATARI_KEY_E,
                after_pc: None
            }]
        );
    }

    #[test]
    fn parses_gated_deferred_key_code_option() {
        let options = parse_options(vec![
            "--key-at-pc-after".to_string(),
            "$A2E0:$B2F5:C".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.deferred_key_codes,
            vec![DeferredKeyCode {
                after_pc: Some(0xA2E0),
                pc: 0xB2F5,
                key_code: ATARI_KEY_C
            }]
        );
    }

    #[test]
    fn parses_action_command_at_pc() {
        let options = parse_options(vec![
            "--action-command-at-pc".to_string(),
            "$B28F:compile".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.deferred_key_codes,
            vec![
                DeferredKeyCode {
                    pc: 0xB28F,
                    key_code: ATARI_KEY_C,
                    after_pc: None
                },
                DeferredKeyCode {
                    pc: 0xB28F,
                    key_code: ATARI_KEY_RETURN,
                    after_pc: None
                }
            ]
        );
    }

    #[test]
    fn parses_source_injection_at_pc() {
        let options = parse_options(vec![
            "--inject-source-at-pc".to_string(),
            "$A2E0:samples/hello.act".to_string(),
            "--dump-editor-lines-at-pc".to_string(),
            "$A2E0".to_string(),
            "--dump-screen-at-pc".to_string(),
            "$A2F0".to_string(),
            "--dump-screen-on-stop".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.deferred_source_injections,
            vec![DeferredSourceInjection {
                pc: 0xA2E0,
                path: PathBuf::from("samples/hello.act")
            }]
        );
        assert_eq!(options.editor_line_dump_pcs, vec![0xA2E0]);
        assert_eq!(options.screen_dump_pcs, vec![0xA2F0]);
        assert!(options.dump_screen_on_stop);
    }

    #[test]
    fn rejects_malformed_source_injection() {
        let err = parse_options(vec![
            "--inject-source-at-pc".to_string(),
            "$A2E0".to_string(),
        ])
        .unwrap_err();

        assert!(err.contains("must be pc:path"));
    }

    #[test]
    fn parses_deferred_monitor_key_option() {
        let options =
            parse_options(vec!["--monitor-key-at-pc".to_string(), "$A2E0".to_string()]).unwrap();

        assert_eq!(
            options.deferred_key_codes,
            vec![DeferredKeyCode {
                pc: 0xA2E0,
                key_code: ACTION_MONITOR_KEY_CODE,
                after_pc: None
            }]
        );
    }

    #[test]
    fn rejects_out_of_range_key_code() {
        let err = parse_options(vec!["--key-code".to_string(), "$100".to_string()]).unwrap_err();

        assert!(err.contains("outside $00-$FF"));
    }

    #[test]
    fn rejects_malformed_deferred_key_code() {
        let err = parse_options(vec!["--key-at-pc".to_string(), "$A2E0".to_string()]).unwrap_err();

        assert!(err.contains("must be pc:key"));
    }
}
