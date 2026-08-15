use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::path::PathBuf;

use action_compiler_vm::{
    ACTION_MONITOR_KEY_CODE, ACTION_OS_PRESET, ACTION_SEGMENT_END_VECTOR, ATARI_KEY_C, ATARI_KEY_E,
    ATARI_KEY_RETURN, ActionEditorLine, ActionSourceInjectionReport, ActionSymbolEntry,
    AddressRange, AtariLoadReport, BusAccess, BusEvent, BusRegion, CioObservation, CioSummary,
    CpuRegisters, CpuStep, ExecutionProfile, Hotpatch, ImageKind, PcTrigger, RunRequest,
    ScheduledAction, ScheduledActionObservation, ScheduledActions, StopReason, TextScreenSnapshot,
    VmConfig, VmRunHooks, VmRunner, action_current_proc_name, decode_action_symbol_tables,
    format_action_symbol_dump_json,
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

struct CliRunHooks<'a> {
    options: &'a CliOptions,
    scheduled_actions: ScheduledActions,
    deferred_source_injections: Vec<DeferredSourceInjection>,
    editor_line_dump_pcs: Vec<u16>,
    screen_dump_pcs: Vec<u16>,
    menu_dump_traps: Vec<MenuDumpTrap>,
    symbol_snapshots: Vec<SymbolSnapshot>,
    action_fixup_trace: ActionFixupTrace,
    action_code_pointer_trace: ActionCodePointerTrace,
    action_call_trace: ActionCallTrace,
}

impl VmRunHooks for CliRunHooks<'_> {
    type Error = String;

    fn before_step(&mut self, vm: &mut action_compiler_vm::CompilerVm) -> Result<(), Self::Error> {
        let pc = vm.cpu().registers().pc;
        let mut source_index = 0;
        while source_index < self.deferred_source_injections.len() {
            if self.deferred_source_injections[source_index].pc == pc {
                let deferred = self.deferred_source_injections.remove(source_index);
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
        while dump_index < self.editor_line_dump_pcs.len() {
            if self.editor_line_dump_pcs[dump_index] == pc {
                let dump_pc = self.editor_line_dump_pcs.remove(dump_index);
                eprintln!("Action! editor lines at PC=${dump_pc:04X}:");
                print_editor_lines(vm.bus())?;
            } else {
                dump_index += 1;
            }
        }

        let mut screen_dump_index = 0;
        while screen_dump_index < self.screen_dump_pcs.len() {
            if self.screen_dump_pcs[screen_dump_index] == pc {
                let dump_pc = self.screen_dump_pcs.remove(screen_dump_index);
                eprintln!("text screen at PC=${dump_pc:04X}:");
                print_text_screen(&vm.bus().text_screen_snapshot(40, 24));
            } else {
                screen_dump_index += 1;
            }
        }

        for trap in &self.menu_dump_traps {
            if trap.pc == pc {
                dump_menu_trap(trap, &vm.cpu().registers(), vm.bus());
            }
        }

        for trigger in &self.options.symbol_snapshot_triggers {
            if trigger.pc == pc {
                let snapshot = capture_symbol_snapshot(trigger, vm.bus());
                if !trigger.skip_empty || snapshot.has_symbols() {
                    self.symbol_snapshots.push(snapshot);
                }
            }
        }

        for observation in self.scheduled_actions.apply_before_step(vm)? {
            match observation {
                ScheduledActionObservation::KeyCodeQueued { pc, key_code } => {
                    eprintln!("queued key ${key_code:02X} at PC=${pc:04X}");
                }
                ScheduledActionObservation::CioInputQueued { pc, byte_count } => {
                    eprintln!("queued {byte_count} scripted CIO byte(s) at PC=${pc:04X}");
                }
                ScheduledActionObservation::ActionSourceInjected { .. } => {}
            }
        }
        Ok(())
    }

    fn after_step(
        &mut self,
        vm: &action_compiler_vm::CompilerVm,
        step: &CpuStep,
    ) -> Result<(), Self::Error> {
        if self.options.should_trace(step.pc) {
            print_step(step);
        }
        self.action_fixup_trace.observe(step, vm.bus());
        self.action_code_pointer_trace.observe(step, vm.bus());
        self.action_call_trace.observe(step, vm.bus());
        Ok(())
    }
}

fn validate_cli_execution(options: &CliOptions) -> Result<(), String> {
    match options.execution_profile {
        ExecutionProfile::SyntheticTest => {
            return Err(
                "synthetic-test is a library-only profile; use VmRunner with caller-installed memory"
                    .to_string(),
            );
        }
        ExecutionProfile::CartridgeObject | ExecutionProfile::StandaloneObject
            if options.load_objects.is_empty() =>
        {
            return Err(format!(
                "{:?} requires at least one --load-object",
                options.execution_profile
            ));
        }
        _ => {}
    }
    options
        .config
        .validate_for_profile(options.execution_profile)
}

fn run_vm(options: CliOptions) -> Result<(), String> {
    let config = options.config.clone();
    validate_cli_execution(&options)?;
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
    if options.load_objects.is_empty() {
        for poke in &options.pokes {
            vm.bus_mut().ram_mut().write(poke.address, poke.value);
        }
    } else {
        vm.prepare_headless_program_environment();
        let mut run_address = None;
        for path in &options.load_objects {
            let bytes = fs::read(path)
                .map_err(|err| format!("failed to read load object `{}`: {err}", path.display()))?;
            let report = vm.load_atari_object(&bytes).map_err(|err| {
                format!(
                    "failed to load Atari object `{}` into VM memory: {err}",
                    path.display()
                )
            })?;
            print_load_object_report(path, &report);
            run_address = report.run_address.or(run_address);
        }
        for poke in &options.pokes {
            vm.bus_mut().ram_mut().write(poke.address, poke.value);
        }
        let run_address = run_address.ok_or_else(|| {
            "loaded object did not contain RUNAD; pass a load file with a RUNAD segment".to_string()
        })?;
        vm.set_pc(run_address);
    }
    if !options.protected_code_ranges.is_empty() {
        vm.protect_code_ranges(&options.protected_code_ranges);
        vm.allow_code_write_ranges(&options.allowed_code_write_ranges);
        eprintln!(
            "protected {} code range(s) from writes",
            options.protected_code_ranges.len()
        );
        if !options.allowed_code_write_ranges.is_empty() {
            eprintln!(
                "allowed {} intentional code write range(s)",
                options.allowed_code_write_ranges.len()
            );
        }
    }
    let action_fixup_trace = ActionFixupTrace::new(options.trace_action_fixups);
    let action_code_pointer_trace =
        ActionCodePointerTrace::new(options.trace_action_code_pointer, vm.bus());
    let action_call_trace = ActionCallTrace::new(
        options.trace_action_calls,
        load_action_call_listings(&options)?,
        load_action_call_maps(&options)?,
    );
    let mut scheduled_actions = ScheduledActions::default();
    for deferred in &options.deferred_scripted_cio_inputs {
        scheduled_actions.schedule(ScheduledAction::queue_cio_input(
            pc_trigger(deferred.pc, deferred.after_pc),
            deferred.bytes.clone(),
        ));
    }
    for deferred in &options.deferred_key_codes {
        scheduled_actions.schedule(ScheduledAction::queue_key_code(
            pc_trigger(deferred.pc, deferred.after_pc),
            deferred.key_code,
        ));
    }
    let mut hooks = CliRunHooks {
        options: &options,
        scheduled_actions,
        deferred_source_injections: options.deferred_source_injections.clone(),
        editor_line_dump_pcs: options.editor_line_dump_pcs.clone(),
        screen_dump_pcs: options.screen_dump_pcs.clone(),
        menu_dump_traps: options.menu_dump_traps.clone(),
        symbol_snapshots: Vec::new(),
        action_fixup_trace,
        action_code_pointer_trace,
        action_call_trace,
    };
    println!(
        "compiler VM loaded {} image(s); start PC=${:04X}",
        vm.images().len(),
        vm.cpu().registers().pc
    );

    if options.max_steps == 0 {
        println!(
            "stopped after 0 step(s), cycles={}, PC=${:04X}",
            vm.cpu().cycles(),
            vm.cpu().registers().pc
        );
        return Ok(());
    }

    let outcome = VmRunner::new(vm).run_with_hooks(
        RunRequest {
            max_steps: options.max_steps,
            stop_after_pc: options.trace_until,
            history_len: options.history_len,
        },
        &mut hooks,
    )?;
    let report = &outcome.report;
    let vm = &outcome.vm;
    let (reason, return_error) = describe_stop(report.stop);

    print_stop_report(
        &reason,
        Some(report.registers),
        Some(&report.history),
        vm.bus().events(),
        vm.bus().cio_summary(),
        vm.bus().cio_observations(),
        vm.bus().cartridge().map(|cart| cart.mapping_info()),
        &hooks.action_fixup_trace,
        &hooks.action_code_pointer_trace,
    );
    print_run_observations(
        vm.bus(),
        options.dump_screen_on_stop,
        &options.memory_dump_ranges,
    );
    capture_final_symbol_snapshot(
        &options,
        &mut hooks.symbol_snapshots,
        vm.cpu().registers().pc,
        vm.bus(),
    );
    write_stop_outputs(&options, vm.bus())?;
    write_symbol_snapshots(&options, &hooks.symbol_snapshots)?;

    if let Some(error) = return_error {
        return Err(error);
    }
    if matches!(report.stop, StopReason::StepLimit { .. }) {
        println!(
            "stopped after {} step(s), cycles={}, PC=${:04X}",
            options.max_steps, report.cycles, report.registers.pc
        );
    }
    Ok(())
}

fn pc_trigger(pc: u16, after_pc: Option<u16>) -> PcTrigger {
    match after_pc {
        Some(after_pc) => PcTrigger::at_after(pc, after_pc),
        None => PcTrigger::at(pc),
    }
}

fn describe_stop(stop: StopReason) -> (String, Option<String>) {
    match stop {
        StopReason::StepLimit { .. } => ("max steps reached".to_string(), None),
        StopReason::PcReached { .. } => ("trace-until reached".to_string(), None),
        StopReason::UnsupportedOpcode { pc, opcode } => {
            let reason = format!("unsupported opcode ${opcode:02X} at ${pc:04X}");
            (reason.clone(), Some(reason))
        }
        StopReason::ProtectedCodeWrite {
            pc,
            address,
            old_value,
            new_value,
            region,
        } => (
            format!(
                "protected code write at ${address:04X}: ${old_value:02X} -> ${new_value:02X} ({region:?}), instruction PC=${pc:04X}"
            ),
            Some(format!(
                "protected code write at ${address:04X}: ${old_value:02X} -> ${new_value:02X}"
            )),
        ),
        StopReason::Halted => ("CPU halted".to_string(), Some("CPU halted".to_string())),
    }
}

fn print_load_object_report(path: &std::path::Path, report: &AtariLoadReport) {
    eprintln!("loaded Atari object `{}`:", path.display());
    for (index, segment) in report.segments.iter().enumerate() {
        eprintln!(
            "  seg {index:02}: ${:04X}-${:04X} len {}",
            segment.start, segment.end, segment.len
        );
    }
    if let Some(run_address) = report.run_address {
        eprintln!("  RUNAD ${run_address:04X}");
    } else {
        eprintln!("  RUNAD <none>");
    }
}

fn capture_symbol_snapshot(
    trigger: &SymbolSnapshotTrigger,
    bus: &action_compiler_vm::Bus,
) -> SymbolSnapshot {
    let dump = decode_action_symbol_tables(bus);
    let proc_name = action_current_proc_name(bus);
    eprintln!(
        "captured symbol snapshot `{}` at PC=${:04X}: proc={}, {} local(s)",
        trigger.label,
        trigger.pc,
        proc_name.as_deref().unwrap_or("<none>"),
        dump.locals.len()
    );
    SymbolSnapshot {
        pc: trigger.pc,
        label: trigger.label.clone(),
        proc_name,
        local_index: dump.local_index,
        locals: dump.locals,
    }
}

fn capture_final_symbol_snapshot(
    options: &CliOptions,
    snapshots: &mut Vec<SymbolSnapshot>,
    pc: u16,
    bus: &action_compiler_vm::Bus,
) {
    if !options.capture_final_symbol_snapshot {
        return;
    }
    let trigger = SymbolSnapshotTrigger {
        pc,
        label: "stop".to_string(),
        skip_empty: true,
    };
    let snapshot = capture_symbol_snapshot(&trigger, bus);
    if !snapshot.has_symbols() {
        return;
    }
    if snapshots
        .last()
        .is_some_and(|last| last.matches_symbols(&snapshot))
    {
        return;
    }
    snapshots.push(snapshot);
}

fn write_symbol_snapshots(
    options: &CliOptions,
    snapshots: &[SymbolSnapshot],
) -> Result<(), String> {
    let Some(path) = &options.symbol_snapshots_path else {
        return Ok(());
    };
    let json = format_symbol_snapshots_json(snapshots);
    fs::write(path, json.as_bytes()).map_err(|err| {
        format!(
            "failed to write symbol snapshots `{}`: {err}",
            path.display()
        )
    })?;
    eprintln!(
        "wrote {} symbol snapshot(s) to {}",
        snapshots.len(),
        path.display()
    );
    Ok(())
}

fn write_stop_outputs(options: &CliOptions, bus: &action_compiler_vm::Bus) -> Result<(), String> {
    write_host_outputs(&options.config, bus)?;
    if let Some(path) = &options.raw_memory_dump_path {
        write_raw_memory_dump(path, bus)?;
    }
    if let Some(path) = &options.symbol_dump_path {
        write_symbol_dump(path, bus)?;
    }
    Ok(())
}

fn write_host_outputs(config: &VmConfig, bus: &action_compiler_vm::Bus) -> Result<(), String> {
    for (name, path) in &config.host_outputs {
        let bytes = bus
            .host_file_bytes(name)
            .ok_or_else(|| format!("host output `{name}` was not registered"))?;
        fs::write(path, bytes)
            .map_err(|err| format!("failed to write host output `{}`: {err}", path.display()))?;
        eprintln!(
            "wrote host output `{name}`: {} byte(s) to {}",
            bytes.len(),
            path.display()
        );
    }
    Ok(())
}

fn write_raw_memory_dump(path: &PathBuf, bus: &action_compiler_vm::Bus) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(0x10000);
    for address in 0..=u16::MAX {
        bytes.push(bus.ram().read(address));
    }
    fs::write(path, &bytes).map_err(|err| {
        format!(
            "failed to write raw memory dump `{}`: {err}",
            path.display()
        )
    })?;
    eprintln!(
        "wrote raw memory dump: {} byte(s) to {}",
        bytes.len(),
        path.display()
    );
    Ok(())
}

fn write_symbol_dump(path: &PathBuf, bus: &action_compiler_vm::Bus) -> Result<(), String> {
    let dump = decode_action_symbol_tables(bus);
    let json = format_action_symbol_dump_json(&dump);
    fs::write(path, json.as_bytes())
        .map_err(|err| format!("failed to write symbol dump `{}`: {err}", path.display()))?;
    eprintln!(
        "wrote symbol dump: {} global(s), {} local(s) to {}",
        dump.globals.len(),
        dump.locals.len(),
        path.display()
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    config: VmConfig,
    execution_profile: ExecutionProfile,
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
    menu_dump_traps: Vec<MenuDumpTrap>,
    memory_dump_ranges: Vec<AddressRange>,
    raw_memory_dump_path: Option<PathBuf>,
    symbol_dump_path: Option<PathBuf>,
    symbol_snapshots_path: Option<PathBuf>,
    symbol_snapshot_triggers: Vec<SymbolSnapshotTrigger>,
    capture_final_symbol_snapshot: bool,
    dump_screen_on_stop: bool,
    trace_action_fixups: bool,
    trace_action_code_pointer: bool,
    trace_action_calls: bool,
    action_call_listings: Vec<PathBuf>,
    action_call_maps: Vec<PathBuf>,
    load_objects: Vec<PathBuf>,
    pokes: Vec<MemoryPoke>,
    protected_code_ranges: Vec<AddressRange>,
    allowed_code_write_ranges: Vec<AddressRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolSnapshotTrigger {
    pc: u16,
    label: String,
    skip_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MenuDumpTrap {
    pc: u16,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolSnapshot {
    pc: u16,
    label: String,
    proc_name: Option<String>,
    local_index: Option<u16>,
    locals: Vec<ActionSymbolEntry>,
}

impl SymbolSnapshot {
    fn has_symbols(&self) -> bool {
        self.proc_name.is_some() || !self.locals.is_empty()
    }

    fn matches_symbols(&self, other: &Self) -> bool {
        self.proc_name == other.proc_name
            && self.local_index == other.local_index
            && self.locals == other.locals
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemoryPoke {
    address: u16,
    value: u8,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            config: VmConfig::default(),
            execution_profile: ExecutionProfile::OriginalCompiler,
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
            menu_dump_traps: Vec::new(),
            memory_dump_ranges: Vec::new(),
            raw_memory_dump_path: None,
            symbol_dump_path: None,
            symbol_snapshots_path: None,
            symbol_snapshot_triggers: Vec::new(),
            capture_final_symbol_snapshot: false,
            dump_screen_on_stop: false,
            trace_action_fixups: false,
            trace_action_code_pointer: false,
            trace_action_calls: false,
            action_call_listings: Vec::new(),
            action_call_maps: Vec::new(),
            load_objects: Vec::new(),
            pokes: Vec::new(),
            protected_code_ranges: Vec::new(),
            allowed_code_write_ranges: Vec::new(),
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
    let mut execution_profile = ExecutionProfile::OriginalCompiler;
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
    let mut menu_dump_traps = Vec::new();
    let mut memory_dump_ranges = Vec::new();
    let mut raw_memory_dump_path = None;
    let mut symbol_dump_path = None;
    let mut symbol_snapshots_path = None;
    let mut symbol_snapshot_triggers = Vec::new();
    let mut capture_final_symbol_snapshot = false;
    let mut dump_screen_on_stop = false;
    let mut trace_action_fixups = false;
    let mut trace_action_code_pointer = false;
    let mut trace_action_calls = false;
    let mut action_call_listings = Vec::new();
    let mut action_call_maps = Vec::new();
    let mut load_objects = Vec::new();
    let mut pokes = Vec::new();
    let mut protected_code_ranges = Vec::new();
    let mut allowed_code_write_ranges = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                index += 1;
                let value = required_value(&args, index, "--profile")?;
                execution_profile = parse_execution_profile(value)?;
            }
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
            "--trace-action-fixups" => {
                trace_action_fixups = true;
            }
            "--trace-action-code-pointer" => {
                trace_action_code_pointer = true;
            }
            "--trace-action-calls" => {
                trace_action_calls = true;
            }
            "--trace-action-calls-from-listing" => {
                index += 1;
                let value = required_value(&args, index, "--trace-action-calls-from-listing")?;
                trace_action_calls = true;
                action_call_listings.push(PathBuf::from(value));
            }
            "--trace-action-calls-from-map" => {
                index += 1;
                let value = required_value(&args, index, "--trace-action-calls-from-map")?;
                trace_action_calls = true;
                action_call_maps.push(PathBuf::from(value));
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
            "--dump-menu-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--dump-menu-at-pc")?;
                menu_dump_traps.push(parse_menu_dump_trap(value)?);
            }
            "--dump-menu-at-proc" => {
                index += 1;
                let value = required_value(&args, index, "--dump-menu-at-proc")?;
                menu_dump_traps.push(parse_menu_dump_trap_from_listing(value)?);
            }
            "--dump-screen-on-stop" => {
                dump_screen_on_stop = true;
            }
            "--dump-range-on-stop" => {
                index += 1;
                let value = required_value(&args, index, "--dump-range-on-stop")?;
                memory_dump_ranges.push(parse_range(value)?);
            }
            "--dump-memory-on-stop" => {
                index += 1;
                let value = required_value(&args, index, "--dump-memory-on-stop")?;
                raw_memory_dump_path = Some(PathBuf::from(value));
            }
            "--dump-symbols-on-stop" => {
                index += 1;
                let value = required_value(&args, index, "--dump-symbols-on-stop")?;
                symbol_dump_path = Some(PathBuf::from(value));
            }
            "--dump-symbol-snapshots-on-stop" => {
                index += 1;
                let value = required_value(&args, index, "--dump-symbol-snapshots-on-stop")?;
                symbol_snapshots_path = Some(PathBuf::from(value));
            }
            "--symbol-snapshot-at-pc" => {
                index += 1;
                let value = required_value(&args, index, "--symbol-snapshot-at-pc")?;
                symbol_snapshot_triggers.push(parse_symbol_snapshot_trigger(value)?);
            }
            "--action-symbol-hooks" => {
                symbol_snapshot_triggers.push(SymbolSnapshotTrigger {
                    pc: ACTION_SEGMENT_END_VECTOR,
                    label: "segvec".to_string(),
                    skip_empty: true,
                });
                capture_final_symbol_snapshot = true;
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
            "--host-output" => {
                index += 1;
                let value = required_value(&args, index, "--host-output")?;
                config.host_outputs.push(parse_host_file_map(value)?);
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
            "--load-object" => {
                index += 1;
                let path = required_value(&args, index, "--load-object")?;
                load_objects.push(PathBuf::from(path));
            }
            "--poke" => {
                index += 1;
                let value = required_value(&args, index, "--poke")?;
                pokes.push(parse_memory_poke(value)?);
            }
            "--protect-code-range" => {
                index += 1;
                let value = required_value(&args, index, "--protect-code-range")?;
                protected_code_ranges.push(parse_range(value)?);
            }
            "--protect-code-from-listing" => {
                index += 1;
                let value = required_value(&args, index, "--protect-code-from-listing")?;
                protected_code_ranges.extend(parse_protected_code_ranges_from_listing(
                    &PathBuf::from(value),
                )?);
            }
            "--allow-code-write-range" => {
                index += 1;
                let value = required_value(&args, index, "--allow-code-write-range")?;
                allowed_code_write_ranges.push(parse_range(value)?);
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(CliOptions {
        config,
        execution_profile,
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
        menu_dump_traps,
        memory_dump_ranges,
        raw_memory_dump_path,
        symbol_dump_path,
        symbol_snapshots_path,
        symbol_snapshot_triggers,
        capture_final_symbol_snapshot,
        dump_screen_on_stop,
        trace_action_fixups,
        trace_action_code_pointer,
        trace_action_calls,
        action_call_listings,
        action_call_maps,
        load_objects,
        pokes,
        protected_code_ranges,
        allowed_code_write_ranges,
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

fn parse_execution_profile(value: &str) -> Result<ExecutionProfile, String> {
    match value {
        "original-compiler" => Ok(ExecutionProfile::OriginalCompiler),
        "cartridge-object" => Ok(ExecutionProfile::CartridgeObject),
        "standalone-object" => Ok(ExecutionProfile::StandaloneObject),
        "synthetic-test" => Ok(ExecutionProfile::SyntheticTest),
        other => Err(format!(
            "unknown execution profile `{other}`; expected original-compiler, cartridge-object, standalone-object, or synthetic-test"
        )),
    }
}

fn parse_hotpatch(value: &str) -> Result<Hotpatch, String> {
    match value {
        "action-q-input" => Ok(Hotpatch::ActionQueuedInput),
        "action-headless-getkey" => Ok(Hotpatch::ActionHeadlessGetkey),
        other => Err(format!("unknown hotpatch `{other}`")),
    }
}

fn parse_symbol_snapshot_trigger(value: &str) -> Result<SymbolSnapshotTrigger, String> {
    let Some((pc, label)) = value.split_once(':') else {
        return Err(format!(
            "symbol snapshot trigger `{value}` must be pc:label"
        ));
    };
    if label.trim().is_empty() {
        return Err("symbol snapshot label must not be empty".to_string());
    }
    Ok(SymbolSnapshotTrigger {
        pc: parse_address(pc)?,
        label: label.to_string(),
        skip_empty: false,
    })
}

fn parse_menu_dump_trap(value: &str) -> Result<MenuDumpTrap, String> {
    let (pc, label) = value
        .split_once(':')
        .map(|(pc, label)| (pc, label.to_string()))
        .unwrap_or((value, "menu".to_string()));
    if label.trim().is_empty() {
        return Err("menu dump label must not be empty".to_string());
    }
    Ok(MenuDumpTrap {
        pc: parse_address(pc)?,
        label,
    })
}

fn parse_menu_dump_trap_from_listing(value: &str) -> Result<MenuDumpTrap, String> {
    let mut parts = value.splitn(3, ':');
    let path = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| format!("menu proc trap `{value}` must be listing:proc[:label]"))?;
    let proc_name = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| format!("menu proc trap `{value}` must be listing:proc[:label]"))?;
    let label = parts
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(proc_name);
    let listing = fs::read_to_string(path)
        .map_err(|err| format!("failed to read listing `{path}` for menu trap: {err}"))?;
    Ok(MenuDumpTrap {
        pc: find_listing_proc_entry(&listing, proc_name)?,
        label: label.to_string(),
    })
}

fn find_listing_proc_entry(listing: &str, proc_name: &str) -> Result<u16, String> {
    for line in listing.lines() {
        let Some((name, entry)) = parse_listing_proc_entry(line)? else {
            continue;
        };
        if name == proc_name {
            return Ok(entry);
        }
    }
    Err(format!("listing did not contain PROC `{proc_name}`"))
}

fn load_action_call_listings(options: &CliOptions) -> Result<HashMap<u16, String>, String> {
    let mut entries = HashMap::new();
    for path in &options.action_call_listings {
        let listing = fs::read_to_string(path).map_err(|err| {
            format!(
                "failed to read action call listing `{}`: {err}",
                path.display()
            )
        })?;
        for line in listing.lines() {
            let Some((name, entry)) = parse_listing_proc_entry(line)? else {
                continue;
            };
            entries.entry(entry).or_insert(name);
        }
    }
    Ok(entries)
}

fn load_action_call_maps(
    options: &CliOptions,
) -> Result<HashMap<u16, ActionMapRoutineSignature>, String> {
    let mut entries = HashMap::new();
    for path in &options.action_call_maps {
        let map = fs::read_to_string(path)
            .map_err(|err| format!("failed to read action call map `{}`: {err}", path.display()))?;
        for line in map.lines() {
            let Some(signature) = parse_action_map_signature_line(line)? else {
                continue;
            };
            entries.entry(signature.address).or_insert(signature);
        }
    }
    Ok(entries)
}

fn parse_listing_proc_entry(line: &str) -> Result<Option<(String, u16)>, String> {
    let Some(header) = line.strip_prefix("; ===== PROC ") else {
        return Ok(None);
    };
    let Some((name, range_text)) = header.split_once(" $") else {
        return Ok(None);
    };
    let Some((start_text, rest)) = range_text.split_once("..") else {
        return Ok(None);
    };
    let entry = if let Some(entry_text) = rest.split(" entry ").nth(1) {
        let Some(entry) = entry_text.split_whitespace().next() else {
            return Err(format!("invalid listing PROC entry in `{line}`"));
        };
        parse_address(entry)?
    } else {
        parse_address(&format!("${start_text}"))?
    };
    Ok(Some((name.to_string(), entry)))
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

fn parse_memory_poke(value: &str) -> Result<MemoryPoke, String> {
    let Some((address, byte)) = value.split_once('=') else {
        return Err(format!("memory poke `{value}` must be address=byte"));
    };
    Ok(MemoryPoke {
        address: parse_address(address)?,
        value: parse_byte(byte)?,
    })
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

fn parse_protected_code_ranges_from_listing(path: &PathBuf) -> Result<Vec<AddressRange>, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read listing `{}`: {err}", path.display()))?;
    let mut ranges = Vec::new();
    for line in text.lines() {
        if let Some(range) = parse_listing_proc_code_range(line)? {
            ranges.push(range);
        }
    }
    if ranges.is_empty() {
        return Err(format!(
            "listing `{}` did not contain any PROC range headers",
            path.display()
        ));
    }
    Ok(ranges)
}

fn parse_listing_proc_code_range(line: &str) -> Result<Option<AddressRange>, String> {
    let Some(header) = line.strip_prefix("; ===== PROC ") else {
        return Ok(None);
    };
    let Some(range_start) = header.find(" $") else {
        return Ok(None);
    };
    let range_text = &header[range_start + 1..];
    let Some((start_text, rest)) = range_text.split_once("..") else {
        return Ok(None);
    };
    let Some(end_text) = rest.split_whitespace().next() else {
        return Ok(None);
    };
    let start = if let Some(entry_text) = rest.split(" entry ").nth(1) {
        let Some(entry) = entry_text.split_whitespace().next() else {
            return Err(format!("invalid listing PROC entry in `{line}`"));
        };
        parse_address(entry)?
    } else {
        parse_address(start_text)?
    };
    let exclusive_end = parse_address(end_text)?;
    let end = exclusive_end
        .checked_sub(1)
        .ok_or_else(|| format!("listing PROC range has zero end in `{line}`"))?;
    if start > end {
        return Err(format!(
            "listing PROC range starts after it ends in `{line}`"
        ));
    }
    Ok(Some(AddressRange { start, end }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionMapRoutineSignature {
    address: u16,
    name: String,
    kind: String,
    params: Vec<ActionMapParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionMapParam {
    name: String,
    type_name: String,
    width: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionRoutineTraceInfo {
    name: String,
    class: Option<String>,
    args: Vec<String>,
    params: Vec<ActionMapParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActionCallFrame {
    name: String,
    expected_return: u16,
}

#[derive(Debug)]
struct ActionCallTrace {
    enabled: bool,
    listing_entries: HashMap<u16, String>,
    map_entries: HashMap<u16, ActionMapRoutineSignature>,
    frames: Vec<ActionCallFrame>,
}

impl ActionCallTrace {
    fn new(
        enabled: bool,
        listing_entries: HashMap<u16, String>,
        map_entries: HashMap<u16, ActionMapRoutineSignature>,
    ) -> Self {
        Self {
            enabled,
            listing_entries,
            map_entries,
            frames: Vec::new(),
        }
    }

    fn observe(&mut self, step: &CpuStep, bus: &action_compiler_vm::Bus) {
        if !self.enabled {
            return;
        }

        if step.opcode == 0x20 {
            self.observe_jsr(step, bus);
            return;
        }

        if step.opcode == 0x60 {
            self.observe_rts(step);
        }
    }

    fn observe_jsr(&mut self, step: &CpuStep, bus: &action_compiler_vm::Bus) {
        let target = bus.ram().read_word(step.pc.wrapping_add(1));
        let Some(info) = self.resolve_target(target, bus) else {
            return;
        };
        let expected_return = step.pc.wrapping_add(3);
        let depth = self.frames.len();
        let before = step.registers_before;
        let after = step.registers_after;
        let stack_lo = bus
            .ram()
            .read(0x0100u16.wrapping_add(u16::from(after.sp.wrapping_add(1))));
        let stack_hi = bus
            .ram()
            .read(0x0100u16.wrapping_add(u16::from(after.sp.wrapping_add(2))));
        let signature = format_action_call_signature(&info);
        let args = format_action_call_args(&info.params, before, bus);
        eprintln!(
            "{:indent$}CALL cyc={} pc=${:04X} target=${:04X} {} {} ret=${:04X} regs_before=A:${:02X} X:${:02X} Y:${:02X} SP:${:02X} entry=A:${:02X} X:${:02X} Y:${:02X} SP:${:02X} stack_ret=${:02X}{:02X}",
            "",
            step.cycles,
            step.pc,
            target,
            signature,
            args,
            expected_return,
            before.a,
            before.x,
            before.y,
            before.sp,
            after.a,
            after.x,
            after.y,
            after.sp,
            stack_hi,
            stack_lo,
            indent = depth * 2
        );
        self.frames.push(ActionCallFrame {
            name: info.name,
            expected_return,
        });
    }

    fn observe_rts(&mut self, step: &CpuStep) {
        let Some(frame) = self.frames.last() else {
            return;
        };
        if step.registers_after.pc != frame.expected_return {
            return;
        }
        let frame = self.frames.pop().expect("call frame");
        let depth = self.frames.len();
        let after = step.registers_after;
        eprintln!(
            "{:indent$}RET  cyc={} pc=${:04X} {} => pc=${:04X} A:${:02X} X:${:02X} Y:${:02X} SP:${:02X}",
            "",
            step.cycles,
            step.pc,
            frame.name,
            after.pc,
            after.a,
            after.x,
            after.y,
            after.sp,
            indent = depth * 2
        );
    }

    fn resolve_target(
        &self,
        target: u16,
        bus: &action_compiler_vm::Bus,
    ) -> Option<ActionRoutineTraceInfo> {
        let symbol = find_action_routine_symbol(target, bus);
        if let Some(signature) = self.map_entries.get(&target) {
            return Some(ActionRoutineTraceInfo {
                name: signature.name.clone(),
                class: Some(signature.kind.clone()),
                args: signature
                    .params
                    .iter()
                    .map(|param| format!("{} {}", param.type_name, param.name))
                    .collect(),
                params: signature.params.clone(),
            });
        }
        if let Some(name) = self.listing_entries.get(&target) {
            return Some(ActionRoutineTraceInfo {
                name: name.clone(),
                class: symbol.as_ref().map(|entry| entry.class.clone()),
                args: symbol
                    .as_ref()
                    .map(|entry| entry.args.clone())
                    .unwrap_or_default(),
                params: Vec::new(),
            });
        }
        symbol.map(|entry| ActionRoutineTraceInfo {
            name: scoped_symbol_name(&entry),
            class: Some(entry.class),
            args: entry.args,
            params: Vec::new(),
        })
    }
}

fn parse_action_map_signature_line(
    line: &str,
) -> Result<Option<ActionMapRoutineSignature>, String> {
    let Some(rest) = line.strip_prefix("signature ") else {
        return Ok(None);
    };
    let mut parts = rest.split_whitespace();
    let address = parts
        .next()
        .ok_or_else(|| format!("invalid action map signature `{line}`"))
        .and_then(parse_address)?;
    let name = parts
        .next()
        .ok_or_else(|| format!("invalid action map signature `{line}`"))?
        .to_string();
    let mut kind = None;
    let mut params = None;
    for part in parts {
        if let Some(value) = part.strip_prefix("kind=") {
            kind = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("params=") {
            params = Some(parse_action_map_params(value)?);
        }
    }
    Ok(Some(ActionMapRoutineSignature {
        address,
        name,
        kind: kind.unwrap_or_else(|| "PROC".to_string()),
        params: params.unwrap_or_default(),
    }))
}

fn parse_action_map_params(value: &str) -> Result<Vec<ActionMapParam>, String> {
    if value == "-" {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|part| {
            let mut fields = part.split(':');
            let name = fields
                .next()
                .filter(|field| !field.is_empty())
                .ok_or_else(|| format!("invalid action map param `{part}`"))?;
            let type_name = fields
                .next()
                .filter(|field| !field.is_empty())
                .ok_or_else(|| format!("invalid action map param `{part}`"))?;
            let width_text = fields
                .next()
                .filter(|field| !field.is_empty())
                .ok_or_else(|| format!("invalid action map param `{part}`"))?;
            if fields.next().is_some() {
                return Err(format!("invalid action map param `{part}`"));
            }
            let width = width_text
                .parse::<u16>()
                .map_err(|_| format!("invalid action map param width `{width_text}`"))?;
            Ok(ActionMapParam {
                name: name.to_string(),
                type_name: type_name.to_string(),
                width,
            })
        })
        .collect()
}

fn format_action_call_args(
    params: &[ActionMapParam],
    regs: CpuRegisters,
    bus: &action_compiler_vm::Bus,
) -> String {
    if params.is_empty() {
        return "args=[]".to_string();
    }
    let mut offset = 0u16;
    let mut rendered = Vec::new();
    for param in params {
        let bytes = (0..param.width)
            .map(|index| action_abi_arg_byte(offset.saturating_add(index), regs, bus))
            .collect::<Vec<_>>();
        let value = bytes
            .iter()
            .enumerate()
            .fold(0u16, |acc, (index, (_, byte))| {
                acc | (u16::from(*byte) << (index * 8))
            });
        let homes = bytes
            .iter()
            .map(|(home, _)| *home)
            .collect::<Vec<_>>()
            .join(":");
        let value_text = if param.width == 1 {
            format!("${:02X}", value & 0x00FF)
        } else {
            format!("${value:04X}")
        };
        rendered.push(format!(
            "{}:{}@{}={}",
            param.name, param.type_name, homes, value_text
        ));
        offset = offset.saturating_add(param.width);
    }
    format!("args=[{}]", rendered.join(", "))
}

fn action_abi_arg_byte(
    offset: u16,
    regs: CpuRegisters,
    bus: &action_compiler_vm::Bus,
) -> (&'static str, u8) {
    match offset {
        0 => ("A", regs.a),
        1 => ("X", regs.x),
        2 => ("Y", regs.y),
        _ => {
            let address = 0x00A0u16.wrapping_add(offset);
            (action_abi_fixed_zp_home(offset), bus.ram().read(address))
        }
    }
}

fn action_abi_fixed_zp_home(offset: u16) -> &'static str {
    match offset {
        3 => "$A3",
        4 => "$A4",
        5 => "$A5",
        6 => "$A6",
        7 => "$A7",
        8 => "$A8",
        9 => "$A9",
        10 => "$AA",
        11 => "$AB",
        12 => "$AC",
        13 => "$AD",
        14 => "$AE",
        15 => "$AF",
        _ => "$A0+",
    }
}

fn find_action_routine_symbol(
    target: u16,
    bus: &action_compiler_vm::Bus,
) -> Option<ActionSymbolEntry> {
    let dump = decode_action_symbol_tables(bus);
    dump.locals
        .into_iter()
        .chain(dump.globals)
        .find(|entry| is_named_routine(entry, target))
}

fn is_named_routine(entry: &ActionSymbolEntry, target: u16) -> bool {
    entry.address == Some(target) && (entry.class.contains("PROC") || entry.class.contains("FUNC"))
}

fn scoped_symbol_name(entry: &ActionSymbolEntry) -> String {
    let scope = match entry.scope {
        action_compiler_vm::ActionSymbolScope::Global => "global",
        action_compiler_vm::ActionSymbolScope::Local => "local",
    };
    format!("{scope}::{}", entry.name)
}

fn format_action_call_signature(info: &ActionRoutineTraceInfo) -> String {
    let mut signature = String::new();
    signature.push_str(&info.name);
    if let Some(class) = &info.class {
        signature.push_str(" [");
        signature.push_str(class);
        signature.push(']');
    }
    signature.push('(');
    for (index, arg) in info.args.iter().enumerate() {
        if index > 0 {
            signature.push_str(", ");
        }
        signature.push_str(arg);
    }
    signature.push(')');
    signature
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

fn dump_menu_trap(trap: &MenuDumpTrap, regs: &CpuRegisters, bus: &action_compiler_vm::Bus) {
    let pointer = u16::from_le_bytes([regs.a, regs.x]);
    eprintln!(
        "menu dump `{}` at PC=${:04X}: input A/X=${:02X}/${:02X} ptr=${:04X} Y=${:02X} A3=${:02X}",
        trap.label,
        trap.pc,
        regs.a,
        regs.x,
        pointer,
        regs.y,
        bus.ram().read(0x00A3)
    );
    eprintln!("  direct pointer candidate:");
    let direct_ok = dump_menu_entries(pointer, bus, 16);
    let indirect = u16::from_le_bytes([
        bus.ram().read(pointer),
        bus.ram().read(pointer.wrapping_add(1)),
    ]);
    if indirect != pointer {
        eprintln!(
            "  indirect CARD candidate from [${pointer:04X}]=${indirect:04X}:{}",
            if direct_ok {
                " (direct already looked valid)"
            } else {
                ""
            }
        );
        dump_menu_entries(indirect, bus, 16);
    }
}

fn dump_menu_entries(start: u16, bus: &action_compiler_vm::Bus, max_entries: usize) -> bool {
    let mut address = start;
    let mut valid = true;
    for index in 0..max_entries {
        let len = bus.ram().read(address);
        if len == 0 {
            eprintln!("  [{index:02}] ${address:04X}: end len=0");
            return valid;
        }
        if len > 64 {
            eprintln!(
                "  [{index:02}] ${address:04X}: malformed len={len:02}; raw={}",
                format_menu_raw_bytes(bus, address, 16)
            );
            return false;
        }
        let text_start = address.wrapping_add(1);
        let mut text_bytes = Vec::with_capacity(len as usize);
        for offset in 0..len {
            text_bytes.push(bus.ram().read(text_start.wrapping_add(offset as u16)));
        }
        let self_ptr_address = text_start.wrapping_add(len as u16);
        let self_ptr = u16::from_le_bytes([
            bus.ram().read(self_ptr_address),
            bus.ram().read(self_ptr_address.wrapping_add(1)),
        ]);
        let key_address = self_ptr_address.wrapping_add(2);
        let key = bus.ram().read(key_address);
        let separator = bus.ram().read(key_address.wrapping_add(1));
        let next = address.wrapping_add(len as u16).wrapping_add(5);
        eprintln!(
            "  [{index:02}] ${address:04X}: len={len:02} text=\"{}\" self=${self_ptr:04X} key=${key:02X} '{}' sep=${separator:02X} next=${next:04X} raw={}",
            format_menu_text(&text_bytes),
            memory_dump_char(key),
            format_menu_raw_entry(bus, address, len)
        );
        if self_ptr != address {
            eprintln!("       warning: self pointer does not match item address");
            valid = false;
        }
        if separator != 0x9A {
            eprintln!("       warning: separator is not $9A");
            valid = false;
        }
        address = next;
    }
    eprintln!("  ... stopped after {max_entries} menu entries without len=0");
    false
}

fn format_menu_raw_entry(bus: &action_compiler_vm::Bus, address: u16, len: u8) -> String {
    let total = u16::from(len).saturating_add(5);
    format_menu_raw_bytes(bus, address, total)
}

fn format_menu_raw_bytes(bus: &action_compiler_vm::Bus, address: u16, total: u16) -> String {
    let mut out = String::new();
    for offset in 0..total {
        if offset != 0 {
            out.push(' ');
        }
        out.push_str(&format!(
            "{:02X}",
            bus.ram().read(address.wrapping_add(offset))
        ));
    }
    out
}

fn format_menu_text(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| memory_dump_char(*byte)).collect()
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

fn format_symbol_snapshots_json(snapshots: &[SymbolSnapshot]) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"snapshots\": ");
    if snapshots.is_empty() {
        out.push_str("[]\n}\n");
        return out;
    }
    out.push_str("[\n");
    for (index, snapshot) in snapshots.iter().enumerate() {
        let comma = if index + 1 == snapshots.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "    {{\"pc\":\"${:04X}\",\"label\":\"{}\",\"proc\":{},\"local_index\":{},\"locals\":[{}]}}{comma}\n",
            snapshot.pc,
            escape_json(&snapshot.label),
            format_json_optional_string(snapshot.proc_name.as_deref()),
            format_json_optional_address(snapshot.local_index),
            format_symbol_entries_inline_json(&snapshot.locals),
        ));
    }
    out.push_str("  ]\n}\n");
    out
}

fn format_symbol_entries_inline_json(entries: &[ActionSymbolEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{{\"slot\":\"${:02X}\",\"name_addr\":\"${:04X}\",\"name\":\"{}\",\"vtype\":\"${:02X}\",\"address\":{},\"class\":\"{}\",\"numargs\":{},\"arg_types_raw\":[{}],\"args\":[{}]}}",
                entry.slot,
                entry.name_addr,
                escape_json(&entry.name),
                entry.vtype,
                format_json_optional_address(entry.address),
                escape_json(&entry.class),
                entry.numargs,
                format_json_byte_array(&entry.arg_types_raw),
                format_json_string_array(&entry.args),
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn format_json_optional_address(address: Option<u16>) -> String {
    address
        .map(|address| format!("\"${address:04X}\""))
        .unwrap_or_else(|| "null".to_string())
}

fn format_json_optional_string(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn format_json_byte_array(values: &[u8]) -> String {
    values
        .iter()
        .map(|value| format!("\"${value:02X}\""))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_json_string_array(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04X}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn cart_word(cartridge: &action_compiler_vm::Cartridge, address: u16) -> u16 {
    let lo = cartridge.read(address).unwrap_or(0xFF);
    let hi = cartridge.read(address.wrapping_add(1)).unwrap_or(0xFF);
    u16::from_le_bytes([lo, hi])
}

#[derive(Debug, Clone)]
struct ActionFixupTrace {
    enabled: bool,
    observations: VecDeque<ActionFixupObservation>,
    pointer_counts: HashMap<u16, u64>,
}

#[derive(Debug, Clone)]
struct ActionFixupObservation {
    cycle: u64,
    pc: u16,
    label: &'static str,
    opcode: u8,
    a_before: u8,
    x_before: u8,
    y_before: u8,
    a_after: u8,
    x_after: u8,
    y_after: u8,
    current: u16,
    next: u16,
    scratch: u16,
    current_bytes: [u8; 5],
    repeated: u64,
}

impl ActionFixupTrace {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            observations: VecDeque::new(),
            pointer_counts: HashMap::new(),
        }
    }

    fn observe(&mut self, step: &CpuStep, bus: &action_compiler_vm::Bus) {
        if !self.enabled {
            return;
        }
        let Some(label) = action_fixup_pc_label(step.pc) else {
            return;
        };

        let current = read_ram_word(bus, 0x00A4);
        let next = read_ram_word(bus, 0x00A0);
        let scratch = read_ram_word(bus, 0x00A2);
        let repeated = {
            let count = self.pointer_counts.entry(current).or_insert(0);
            *count += 1;
            *count
        };
        let current_bytes = read_ram_window(bus, current.wrapping_sub(2));
        let before = step.registers_before;
        let after = step.registers_after;
        self.observations.push_back(ActionFixupObservation {
            cycle: step.cycles,
            pc: step.pc,
            label,
            opcode: step.opcode,
            a_before: before.a,
            x_before: before.x,
            y_before: before.y,
            a_after: after.a,
            x_after: after.x,
            y_after: after.y,
            current,
            next,
            scratch,
            current_bytes,
            repeated,
        });
        if self.observations.len() > 64 {
            self.observations.pop_front();
        }
    }

    fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    fn hot_pointers(&self) -> Vec<(u16, u64)> {
        let mut pointers = self
            .pointer_counts
            .iter()
            .map(|(pointer, count)| (*pointer, *count))
            .collect::<Vec<_>>();
        pointers.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        pointers.truncate(8);
        pointers
    }
}

fn action_fixup_pc_label(pc: u16) -> Option<&'static str> {
    match pc {
        0xA7E9 => Some("loop"),
        0xA836 => Some("advance"),
        0xA874 => Some("next"),
        0xA88A => Some("patch"),
        _ => None,
    }
}

fn read_ram_word(bus: &action_compiler_vm::Bus, address: u16) -> u16 {
    let lo = bus.ram().read(address);
    let hi = bus.ram().read(address.wrapping_add(1));
    u16::from_le_bytes([lo, hi])
}

fn read_ram_window(bus: &action_compiler_vm::Bus, start: u16) -> [u8; 5] {
    [
        bus.ram().read(start),
        bus.ram().read(start.wrapping_add(1)),
        bus.ram().read(start.wrapping_add(2)),
        bus.ram().read(start.wrapping_add(3)),
        bus.ram().read(start.wrapping_add(4)),
    ]
}

#[derive(Debug, Clone)]
struct ActionCodePointerTrace {
    enabled: bool,
    last_pointer: u16,
    last_page: u8,
    last_region: BusRegion,
    observations: VecDeque<ActionCodePointerObservation>,
}

#[derive(Debug, Clone)]
struct ActionCodePointerObservation {
    cycle: u64,
    pc: u16,
    pointer: u16,
    codebase: u16,
    region: BusRegion,
    portb: u8,
    reason: &'static str,
}

impl ActionCodePointerTrace {
    fn new(enabled: bool, bus: &action_compiler_vm::Bus) -> Self {
        let pointer = action_code_pointer(bus);
        let region = bus.visible_region(pointer);
        let mut trace = Self {
            enabled,
            last_pointer: pointer,
            last_page: (pointer >> 8) as u8,
            last_region: region,
            observations: VecDeque::new(),
        };
        if enabled {
            trace.push(0, 0, bus, "initial");
        }
        trace
    }

    fn observe(&mut self, step: &CpuStep, bus: &action_compiler_vm::Bus) {
        if !self.enabled {
            return;
        }

        let pointer = action_code_pointer(bus);
        let page = (pointer >> 8) as u8;
        let region = bus.visible_region(pointer);
        let reason = if crossed_boundary(self.last_pointer, pointer, 0x5000) {
            Some("crossed $5000")
        } else if crossed_boundary(self.last_pointer, pointer, 0x5800) {
            Some("crossed $5800")
        } else if region != self.last_region {
            Some("visible region changed")
        } else if page != self.last_page {
            Some("page changed")
        } else {
            None
        };

        self.last_pointer = pointer;
        self.last_page = page;
        self.last_region = region;

        if let Some(reason) = reason {
            self.push(step.cycles, step.pc, bus, reason);
        }
    }

    fn push(&mut self, cycle: u64, pc: u16, bus: &action_compiler_vm::Bus, reason: &'static str) {
        self.observations.push_back(ActionCodePointerObservation {
            cycle,
            pc,
            pointer: action_code_pointer(bus),
            codebase: action_codebase(bus),
            region: bus.visible_region(action_code_pointer(bus)),
            portb: bus.io().portb(),
            reason,
        });
        if self.observations.len() > 64 {
            self.observations.pop_front();
        }
    }

    fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    fn final_pointer(&self) -> u16 {
        self.last_pointer
    }

    fn final_region(&self) -> BusRegion {
        self.last_region
    }
}

fn action_code_pointer(bus: &action_compiler_vm::Bus) -> u16 {
    read_ram_word(bus, 0x000E)
}

fn action_codebase(bus: &action_compiler_vm::Bus) -> u16 {
    read_ram_word(bus, 0x0491)
}

fn crossed_boundary(previous: u16, current: u16, boundary: u16) -> bool {
    previous < boundary && current >= boundary
}

fn print_stop_report(
    reason: &str,
    registers: Option<CpuRegisters>,
    history: Option<&[CpuStep]>,
    events: &[BusEvent],
    cio_summary: &CioSummary,
    cio_observations: &VecDeque<CioObservation>,
    cartridge: Option<action_compiler_vm::CartridgeMappingInfo>,
    action_fixup_trace: &ActionFixupTrace,
    action_code_pointer_trace: &ActionCodePointerTrace,
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
        for step in history {
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
    if cio_summary.calls > 0 {
        eprintln!(
            "CIO summary: calls={} handled={} passthrough={} open={} close={} status={} read={} write={} eof={} bytes_read={} bytes_written={}",
            cio_summary.calls,
            cio_summary.handled,
            cio_summary.passthrough,
            cio_summary.opens,
            cio_summary.closes,
            cio_summary.statuses,
            cio_summary.reads,
            cio_summary.writes,
            cio_summary.eof,
            cio_summary.bytes_read,
            cio_summary.bytes_written
        );
        eprintln!("recent CIO:");
        for observation in cio_observations
            .iter()
            .rev()
            .take(32)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            eprintln!(
                "  cyc={} dcyc={} x=${:02X} ch={} cmd=${:02X} ret=${:04X} aux=${:02X}/${:02X} buf=${:04X} len={} dev={} {} A={} Y={} {}",
                observation.cycle,
                observation
                    .delta_cycles
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                observation.x,
                observation.channel.unwrap_or(0xFF),
                observation.command,
                observation.return_pc,
                observation.aux1,
                observation.aux2,
                observation.buffer,
                observation.length,
                observation.device_before.as_deref().unwrap_or("-"),
                if observation.handled {
                    "handled"
                } else {
                    "pass"
                },
                observation
                    .result_a
                    .map(|value| format!("${value:02X}"))
                    .unwrap_or_else(|| "--".to_string()),
                observation
                    .result_y
                    .map(|value| format!("${value:02X}"))
                    .unwrap_or_else(|| "--".to_string()),
                format_cio_detail(observation)
            );
        }
    }
    print_action_fixup_trace(action_fixup_trace);
    print_action_code_pointer_trace(action_code_pointer_trace);
}

fn print_action_code_pointer_trace(trace: &ActionCodePointerTrace) {
    if trace.is_empty() {
        return;
    }
    eprintln!(
        "Action! code pointer: *=${:04X} visible={:?}",
        trace.final_pointer(),
        trace.final_region()
    );
    eprintln!("  recent * changes:");
    for observation in &trace.observations {
        eprintln!(
            "  cyc={} pc=${:04X} *=${:04X} codebase=${:04X} visible={:?} PORTB=${:02X} {}",
            observation.cycle,
            observation.pc,
            observation.pointer,
            observation.codebase,
            observation.region,
            observation.portb,
            observation.reason
        );
    }
}

fn print_action_fixup_trace(trace: &ActionFixupTrace) {
    if trace.is_empty() {
        return;
    }
    eprintln!("Action! fixup trace:");
    let hot = trace.hot_pointers();
    if !hot.is_empty() {
        let hot = hot
            .iter()
            .map(|(pointer, count)| format!("${pointer:04X}x{count}"))
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("  hot pointers: {hot}");
    }
    eprintln!("  recent fixups:");
    for observation in trace
        .observations
        .iter()
        .rev()
        .take(32)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        eprintln!(
            "  cyc={} pc=${:04X} {:<7} op=${:02X} A/X/Y ${:02X}/${:02X}/${:02X}->${:02X}/${:02X}/${:02X} cur=${:04X} next=${:04X} scratch=${:04X} rep={} bytes=[{:02X} {:02X} {:02X} {:02X} {:02X}]",
            observation.cycle,
            observation.pc,
            observation.label,
            observation.opcode,
            observation.a_before,
            observation.x_before,
            observation.y_before,
            observation.a_after,
            observation.x_after,
            observation.y_after,
            observation.current,
            observation.next,
            observation.scratch,
            observation.repeated,
            observation.current_bytes[0],
            observation.current_bytes[1],
            observation.current_bytes[2],
            observation.current_bytes[3],
            observation.current_bytes[4],
        );
    }
}

fn format_cio_detail(observation: &CioObservation) -> String {
    match observation.preview.as_deref() {
        Some(preview) => format!("{} read=\"{}\"", observation.detail, preview),
        None => observation.detail.clone(),
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
         --profile <name>     Execution profile: original-compiler (default),\n  \
                              cartridge-object, or standalone-object\n  \
         --cart <path>        Load an Action! cartridge image\n  \
         --cart-base <addr>   Cartridge base address, default $A000\n  \
         --os <path>          Load an Atari OS ROM image at $C000\n  \
         --os-base <addr>     OS ROM base address, default $C000\n  \
         --max-cycles <n>     Run at most n CPU steps, default 1000\n  \
         --trace-pc           Print one line per executed instruction\n  \
         --trace-range <a:b>  Print instructions with PC inside the range\n  \
         --trace-until <addr> Stop after executing an instruction at addr\n  \
         --trace-action-fixups\n  \
                              Summarize Action! compiler branch-fixup loop activity on stop\n  \
         --trace-action-code-pointer\n  \
                              Summarize Action!'s code pointer $0E/$0F and visible memory region\n  \
         --trace-action-calls Print named Action! JSR/RTS call boundaries\n  \
         --trace-action-calls-from-listing <path>\n  \
                              Name traced calls from an actionc listing; repeatable\n  \
         --trace-action-calls-from-map <path>\n  \
                              Decode traced calls from an actionc --emit-map file; repeatable\n  \
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
         --dump-menu-at-pc <pc[:label]>\n  \
                              Dump Action/TN popup menu entries from incoming A/X pointer at pc\n  \
         --dump-menu-at-proc <listing:proc[:label]>\n  \
                              Dump Action/TN popup menu entries at a PROC entry from listing\n  \
         --dump-screen-on-stop\n  \
                              Dump decoded 40x24 text screen in stop reports\n  \
         --dump-range-on-stop <a:b>\n  \
                              Dump RAM bytes in range when execution stops\n  \
         --dump-memory-on-stop <path>\n  \
                              Write raw 64K RAM image when execution stops\n  \
         --dump-symbols-on-stop <path>\n  \
                              Write decoded Action! symbol tables as JSON when execution stops\n  \
         --dump-symbol-snapshots-on-stop <path>\n  \
                              Write captured Action! local symbol snapshots as JSON when execution stops\n  \
         --symbol-snapshot-at-pc <pc:label>\n  \
                              Capture local symbols whenever execution reaches pc\n  \
         --action-symbol-hooks\n  \
                              Capture local symbols at Action!'s segment-end vector ($04C6)\n  \
         --source <path>      Source file reserved for the future compiler harness\n  \
         --host-file <n:path> Register a host file visible as H:n and D:n\n  \
         --host-output <n:path>\n  \
         Register writable host file H:n/D:n and save it to path on stop\n  \
         --load-object <path> Load Atari load-format object into RAM and run RUNAD\n  \
         --poke <addr=byte>   Write one RAM byte before execution; repeatable\n  \
         --protect-code-range <a:b>\n  \
                              Halt if CPU writes into this RAM code range\n  \
         --protect-code-from-listing <path>\n  \
                              Halt if CPU writes into PROC ranges from an actionc source listing\n  \
         --allow-code-write-range <a:b>\n  \
                              Allow intentional writes inside protected code ranges\n  \
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
    fn parses_host_output_option() {
        let options = parse_options(vec![
            "--host-output".to_string(),
            "FUNCTIONS.COM:/tmp/functions.com".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.config.host_outputs,
            vec![(
                "FUNCTIONS.COM".to_string(),
                PathBuf::from("/tmp/functions.com")
            )]
        );
    }

    #[test]
    fn parses_action_fixup_trace_option() {
        let options = parse_options(vec!["--trace-action-fixups".to_string()]).unwrap();

        assert!(options.trace_action_fixups);
    }

    #[test]
    fn parses_action_code_pointer_trace_option() {
        let options = parse_options(vec!["--trace-action-code-pointer".to_string()]).unwrap();

        assert!(options.trace_action_code_pointer);
    }

    #[test]
    fn parses_action_call_trace_option() {
        let options = parse_options(vec!["--trace-action-calls".to_string()]).unwrap();

        assert!(options.trace_action_calls);
        assert!(options.action_call_listings.is_empty());
        assert!(options.action_call_maps.is_empty());
    }

    #[test]
    fn parses_action_call_trace_listing_option() {
        let options = parse_options(vec![
            "--trace-action-calls-from-listing".to_string(),
            "tn.lst".to_string(),
        ])
        .unwrap();

        assert!(options.trace_action_calls);
        assert_eq!(options.action_call_listings, vec![PathBuf::from("tn.lst")]);
    }

    #[test]
    fn parses_action_call_trace_map_option() {
        let options = parse_options(vec![
            "--trace-action-calls-from-map".to_string(),
            "tn.map".to_string(),
        ])
        .unwrap();

        assert!(options.trace_action_calls);
        assert_eq!(options.action_call_maps, vec![PathBuf::from("tn.map")]);
    }

    #[test]
    fn formats_action_call_signature() {
        let signature = format_action_call_signature(&ActionRoutineTraceInfo {
            name: "Convert".to_string(),
            class: Some("PROC".to_string()),
            args: vec!["BYTE c".to_string(), "CARD ptr".to_string()],
            params: Vec::new(),
        });

        assert_eq!(signature, "Convert [PROC](BYTE c, CARD ptr)");
    }

    #[test]
    fn parses_action_map_signature_line() {
        let signature = parse_action_map_signature_line(
            "signature $3210 Convert kind=PROC params=c:BYTE:1,ptr:CARD:2 return=-",
        )
        .unwrap()
        .unwrap();

        assert_eq!(signature.address, 0x3210);
        assert_eq!(signature.name, "Convert");
        assert_eq!(signature.kind, "PROC");
        assert_eq!(
            signature.params,
            vec![
                ActionMapParam {
                    name: "c".to_string(),
                    type_name: "BYTE".to_string(),
                    width: 1,
                },
                ActionMapParam {
                    name: "ptr".to_string(),
                    type_name: "CARD".to_string(),
                    width: 2,
                }
            ]
        );
    }

    #[test]
    fn parses_protected_code_range_option() {
        let options = parse_options(vec![
            "--protect-code-range".to_string(),
            "$310C:$325F".to_string(),
            "--allow-code-write-range".to_string(),
            "$2F7D:$2F7E".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.protected_code_ranges,
            vec![AddressRange {
                start: 0x310C,
                end: 0x325F,
            }]
        );
        assert_eq!(
            options.allowed_code_write_ranges,
            vec![AddressRange {
                start: 0x2F7D,
                end: 0x2F7E,
            }]
        );
    }

    #[test]
    fn parses_listing_proc_range_from_entry_to_end() {
        let line = "; ===== PROC Window $3105..$325F entry $310C =====";

        assert_eq!(
            parse_listing_proc_code_range(line).unwrap(),
            Some(AddressRange {
                start: 0x310C,
                end: 0x325E,
            })
        );
    }

    #[test]
    fn parses_machine_listing_proc_range_from_start_to_end() {
        let line = "; ===== PROC Block $2ED2..$2F19 =====";

        assert_eq!(
            parse_listing_proc_code_range(line).unwrap(),
            Some(AddressRange {
                start: 0x2ED2,
                end: 0x2F18,
            })
        );
    }

    #[test]
    fn parses_listing_proc_entry() {
        let line = "; ===== PROC Items $35A7..$35EA entry $35AA =====";

        assert_eq!(
            parse_listing_proc_entry(line).unwrap(),
            Some(("Items".to_string(), 0x35AA))
        );
    }

    #[test]
    fn parses_listing_proc_entry_without_explicit_entry() {
        let line = "; ===== PROC r_2 $2C02..$2C24 =====";

        assert_eq!(
            parse_listing_proc_entry(line).unwrap(),
            Some(("r_2".to_string(), 0x2C02))
        );
    }

    #[test]
    fn parses_menu_dump_trap_from_listing_proc() {
        let path = env::temp_dir().join(format!(
            "action-compiler-vm-menu-trap-{}.lst",
            std::process::id()
        ));
        fs::write(&path, "; ===== PROC Items $35A7..$35EA entry $35AA =====\n").unwrap();

        let options = parse_options(vec![
            "--dump-menu-at-proc".to_string(),
            format!("{}:Items", path.display()),
        ])
        .unwrap();

        assert_eq!(
            options.menu_dump_traps,
            vec![MenuDumpTrap {
                pc: 0x35AA,
                label: "Items".to_string()
            }]
        );
        let _ = fs::remove_file(path);
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
            "--dump-menu-at-pc".to_string(),
            "$35AA:Items".to_string(),
            "--dump-screen-on-stop".to_string(),
            "--dump-memory-on-stop".to_string(),
            "memory.bin".to_string(),
            "--dump-symbols-on-stop".to_string(),
            "symbols.json".to_string(),
            "--dump-symbol-snapshots-on-stop".to_string(),
            "symbol-snapshots.json".to_string(),
            "--symbol-snapshot-at-pc".to_string(),
            "$04C6:segvec".to_string(),
            "--action-symbol-hooks".to_string(),
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
        assert_eq!(
            options.menu_dump_traps,
            vec![MenuDumpTrap {
                pc: 0x35AA,
                label: "Items".to_string()
            }]
        );
        assert_eq!(
            options.raw_memory_dump_path,
            Some(PathBuf::from("memory.bin"))
        );
        assert_eq!(
            options.symbol_dump_path,
            Some(PathBuf::from("symbols.json"))
        );
        assert_eq!(
            options.symbol_snapshots_path,
            Some(PathBuf::from("symbol-snapshots.json"))
        );
        assert_eq!(
            options.symbol_snapshot_triggers,
            vec![
                SymbolSnapshotTrigger {
                    pc: 0x04C6,
                    label: "segvec".to_string(),
                    skip_empty: false
                },
                SymbolSnapshotTrigger {
                    pc: ACTION_SEGMENT_END_VECTOR,
                    label: "segvec".to_string(),
                    skip_empty: true
                }
            ]
        );
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

    #[test]
    fn characterizes_successful_cli_stop_descriptions() {
        assert_eq!(
            describe_stop(StopReason::StepLimit { max_steps: 10 }),
            ("max steps reached".to_string(), None)
        );
        assert_eq!(
            describe_stop(StopReason::PcReached { pc: 0x3456 }),
            ("trace-until reached".to_string(), None)
        );
    }

    #[test]
    fn parses_execution_profiles_and_defaults_to_original_compiler() {
        assert_eq!(
            parse_options(Vec::new()).unwrap().execution_profile,
            ExecutionProfile::OriginalCompiler
        );
        let options = parse_options(vec![
            "--profile".to_string(),
            "standalone-object".to_string(),
            "--load-object".to_string(),
            "program.com".to_string(),
        ])
        .unwrap();

        assert_eq!(
            options.execution_profile,
            ExecutionProfile::StandaloneObject
        );
        validate_cli_execution(&options).unwrap();
    }

    #[test]
    fn validates_cli_profile_specific_requirements() {
        let mut options = CliOptions {
            execution_profile: ExecutionProfile::CartridgeObject,
            ..CliOptions::default()
        };
        assert!(
            validate_cli_execution(&options)
                .unwrap_err()
                .contains("--load-object")
        );

        options.execution_profile = ExecutionProfile::SyntheticTest;
        assert!(
            validate_cli_execution(&options)
                .unwrap_err()
                .contains("library-only")
        );

        let error =
            parse_options(vec!["--profile".to_string(), "unknown".to_string()]).unwrap_err();
        assert!(error.contains("unknown execution profile"));
    }

    #[test]
    fn characterizes_failed_cli_stop_descriptions() {
        assert_eq!(
            describe_stop(StopReason::UnsupportedOpcode {
                pc: 0x3456,
                opcode: 0x02,
            }),
            (
                "unsupported opcode $02 at $3456".to_string(),
                Some("unsupported opcode $02 at $3456".to_string())
            )
        );
        assert_eq!(
            describe_stop(StopReason::ProtectedCodeWrite {
                pc: 0x3456,
                address: 0x3005,
                old_value: 0xEA,
                new_value: 0x42,
                region: BusRegion::Ram,
            }),
            (
                "protected code write at $3005: $EA -> $42 (Ram), instruction PC=$3456".to_string(),
                Some("protected code write at $3005: $EA -> $42".to_string())
            )
        );
        assert_eq!(
            describe_stop(StopReason::Halted),
            ("CPU halted".to_string(), Some("CPU halted".to_string()))
        );
    }
}
