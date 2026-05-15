use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;

pub const RAM_SIZE: usize = 0x10000;
pub const DEFAULT_CART_BASE: u16 = 0xA000;
pub const OS_ROM_BASE: u16 = 0xC000;
pub const IO_BASE: u16 = 0xD000;
pub const IO_SIZE: usize = 0x0800;
pub const SELF_TEST_BASE: u16 = 0x5000;
pub const SELF_TEST_SIZE: usize = 0x0800;
pub const BOOTQ_SUCCESSFUL_BOOT_FLAG: u16 = 0x0009;
pub const DOSVEC_START_VECTOR: u16 = 0x000A;
pub const PORTB: u16 = 0xD301;
pub const PORTB_SELF_TEST_DISABLE: u8 = 0x80;
pub const ANTIC_VCOUNT: u16 = 0xD40B;
pub const RTCLOK_LOW: u16 = 0x0014;
pub const KBCODE_PRIOR_KEY_CODE: u16 = 0x02F2;
pub const CH_KEY_CODE: u16 = 0x02FC;
pub const RMARGIN: u16 = 0x0053;
pub const ROWCRS: u16 = 0x0054;
pub const COLCRS: u16 = 0x0055;
pub const RAMTOP_MEMORY_TOP_PAGE: u16 = 0x006A;
pub const SAVMSC_SCREEN_MEMORY_POINTER: u16 = 0x0058;
pub const SDLSTL_DISPLAY_LIST_POINTER: u16 = 0x0230;
pub const MEMTOP_OS_TOP_OF_FREE_MEMORY: u16 = 0x02E5;
pub const DEFAULT_HEADLESS_RAMTOP_PAGE: u8 = 0xA0;
pub const DEFAULT_HEADLESS_MEMTOP: u16 = 0x9C1F;
pub const DEFAULT_HEADLESS_SCREEN: u16 = 0x9C40;
pub const ACTION_MONITOR_KEY_CODE: u8 = 0xE5;
pub const ATARI_KEY_RETURN: u8 = 0x0C;
pub const ATARI_KEY_C: u8 = 0x12;
pub const ATARI_KEY_E: u8 = 0x2A;
pub const ACTION_AFBASE: u16 = 0x0080;
pub const ACTION_CHOFF: u16 = 0x008D;
pub const ACTION_LNUM: u16 = 0x008E;
pub const ACTION_DIRTY: u16 = 0x008F;
pub const ACTION_TOP: u16 = 0x0090;
pub const ACTION_BOT: u16 = 0x0092;
pub const ACTION_CUR: u16 = 0x0094;
pub const ACTION_BUF: u16 = 0x009B;
pub const ACTION_DIRTYF: u16 = 0x00C3;
pub const ACTION_GLOBAL_SYMBOL_TABLE_POINTER: u16 = 0x00B1;
pub const ACTION_LOCAL_SYMBOL_TABLE_POINTER: u16 = 0x00B3;
pub const ACTION_VARS_W1: u16 = 0x0480;
pub const ACTION_VARS_TOP1: u16 = 0x048F;
pub const ACTION_LINEMAX: u16 = 0x04CF;
pub const ACTION_WINDOW_TOP_OFFSET: u16 = 4;
pub const ACTION_WINDOW_BOT_OFFSET: u16 = 6;
pub const ACTION_WINDOW_CUR_OFFSET: u16 = 8;
pub const ACTION_LINE_HEADER_SIZE: u16 = 6;
pub const ACTION_LINE_ALLOC_OVERHEAD: u16 = 7;
pub const RECVDN_RECEIVE_DONE_FLAG: u16 = 0x0039;
pub const XMTDON_TRANSMISSION_DONE_FLAG: u16 = 0x003A;
pub const TIMFLG_TIMEOUT_FLAG: u16 = 0x0317;
pub const CONSOL: u16 = 0xD01F;
pub const CONSOL_NO_KEYS: u8 = 0x07;
pub const SEROUT_SERIAL_OUTPUT: u16 = 0xD20D;
pub const CIOV: u16 = 0xE456;
pub const IOCB_DEVICE_BASE: u16 = 0x0341;
pub const IOCB_COMMAND_BASE: u16 = 0x0342;
pub const IOCB_BUFFER_BASE: u16 = 0x0344;
pub const IOCB_LENGTH_BASE: u16 = 0x0348;
pub const IOCB_AUX1_BASE: u16 = 0x034A;
pub const IOCB_AUX2_BASE: u16 = 0x034B;
pub const CIO_COMMAND_OPEN: u8 = 0x03;
pub const CIO_COMMAND_GETREC: u8 = 0x05;
pub const CIO_COMMAND_GETCHR: u8 = 0x07;
pub const CIO_COMMAND_PUTREC: u8 = 0x09;
pub const CIO_COMMAND_PUTCHR: u8 = 0x0B;
pub const CIO_COMMAND_CLOSE: u8 = 0x0C;
pub const CIO_COMMAND_STATUS: u8 = 0x0D;
pub const CARTCS_COLDSTART_VECTOR: u16 = 0xBFFA;
pub const OSS_BANKED_8K_WINDOW_SIZE: usize = 0x2000;
pub const OSS_TYPE_15_BANK_SIZE: usize = 0x1000;
pub const OSS_TYPE_15_FIXED_BASE: u16 = 0xB000;
pub const CAR_HEADER_SIZE: usize = 16;
pub const CAR_MAGIC: &[u8; 4] = b"CART";
pub const RESET_VECTOR: u16 = 0xFFFC;
pub const ACTION_OS_PRESET: MappingPreset = MappingPreset {
    name: "action-os",
    cartridge_base: DEFAULT_CART_BASE,
    os_base: OS_ROM_BASE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingPreset {
    pub name: &'static str,
    pub cartridge_base: u16,
    pub os_base: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmConfig {
    pub cartridge: Option<PathBuf>,
    pub cartridge_base: u16,
    pub os_rom: Option<PathBuf>,
    pub os_base: u16,
    pub source: Option<PathBuf>,
    pub extra_images: Vec<(ImageKind, PathBuf, u16)>,
    pub hotpatches: Vec<Hotpatch>,
    pub host_files: Vec<(String, PathBuf)>,
    pub host_outputs: Vec<(String, PathBuf)>,
    pub trace_cio: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            cartridge: None,
            cartridge_base: ACTION_OS_PRESET.cartridge_base,
            os_rom: None,
            os_base: ACTION_OS_PRESET.os_base,
            source: None,
            extra_images: Vec::new(),
            hotpatches: Vec::new(),
            host_files: Vec::new(),
            host_outputs: Vec::new(),
            trace_cio: false,
        }
    }
}

impl VmConfig {
    pub fn apply_preset(&mut self, preset: MappingPreset) {
        self.cartridge_base = preset.cartridge_base;
        self.os_base = preset.os_base;
    }

    pub fn validate_for_execution(&self) -> Result<(), String> {
        if self.cartridge.is_none() {
            return Err("run requires --cart with an Action! cartridge ROM".to_string());
        }
        if self.os_rom.is_none() {
            return Err("run currently requires --os with an Atari OS ROM".to_string());
        }
        Ok(())
    }

    pub fn load(&self) -> Result<CompilerVm, String> {
        let mut vm = CompilerVm::default();

        if let Some(path) = &self.cartridge {
            vm.load_image(ImageKind::Cartridge, path.clone(), self.cartridge_base)?;
        }

        if let Some(path) = &self.os_rom {
            vm.load_image(ImageKind::Rom, path.clone(), self.os_base)?;
        }

        for (kind, path, base) in &self.extra_images {
            vm.load_image(*kind, path.clone(), *base)?;
        }

        for hotpatch in &self.hotpatches {
            vm.apply_hotpatch(*hotpatch)?;
        }

        if let Some(path) = &self.source {
            vm.source = Some(
                fs::read(path)
                    .map_err(|err| format!("failed to read source `{}`: {err}", path.display()))?,
            );
        }

        for (name, path) in &self.host_files {
            let bytes = fs::read(path)
                .map_err(|err| format!("failed to read host file `{}`: {err}", path.display()))?;
            vm.bus_mut().add_host_file(name, bytes);
        }
        for (name, _) in &self.host_outputs {
            vm.bus_mut().add_host_output(name);
        }
        vm.bus_mut().set_trace_cio(self.trace_cio);

        Ok(vm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotpatch {
    ActionQueuedInput,
    ActionHeadlessGetkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotpatchReport {
    pub patch: Hotpatch,
    pub payload_offset: usize,
    pub old_value: u8,
    pub new_value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Ram,
    Rom,
    Cartridge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedImage {
    pub kind: ImageKind,
    pub path: PathBuf,
    pub base: u16,
    pub metadata: ImageMetadata,
    pub car_header: Option<CarHeader>,
    pub cartridge_mapping: Option<CartridgeMappingInfo>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMetadata {
    pub size: usize,
    pub base: u16,
    pub end: u16,
    pub checksum16: u16,
    pub crc32: u32,
}

impl ImageMetadata {
    pub fn from_bytes(base: u16, bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() {
            return Err("image is empty".to_string());
        }

        let end = mapped_end(base, bytes.len())?;
        Ok(Self {
            size: bytes.len(),
            base,
            end,
            checksum16: checksum16(bytes),
            crc32: crc32(bytes),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarHeader {
    pub cartridge_type: u32,
    pub checksum: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CartridgeMappingInfo {
    pub window_start: u16,
    pub window_end: u16,
    pub bank_size: usize,
    pub bank_count: usize,
    pub active_bank: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerVm {
    bus: Bus,
    images: Vec<LoadedImage>,
    source: Option<Vec<u8>>,
    cpu: Cpu,
}

impl Default for CompilerVm {
    fn default() -> Self {
        Self {
            bus: Bus::default(),
            images: Vec::new(),
            source: None,
            cpu: Cpu::default(),
        }
    }
}

impl CompilerVm {
    pub fn images(&self) -> &[LoadedImage] {
        &self.images
    }

    pub fn memory(&self) -> &Memory {
        self.bus.ram()
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    pub fn source(&self) -> Option<&[u8]> {
        self.source.as_deref()
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn reset_cpu(&mut self) {
        self.cpu.reset(&mut self.bus);
    }

    pub fn step_cpu(&mut self) -> Result<CpuStep, CpuError> {
        if let Some(target) = self
            .bus
            .take_disk_boot_cartridge_redirect(self.cpu.registers.pc)
        {
            self.cpu.set_pc(target);
        }
        self.cpu.step(&mut self.bus)
    }

    pub fn apply_hotpatch(&mut self, hotpatch: Hotpatch) -> Result<HotpatchReport, String> {
        let (report, payload, mapping) = {
            let Some(cartridge) = self.bus.cartridge_mut() else {
                return Err("hotpatch requires a loaded cartridge".to_string());
            };
            let report = cartridge.apply_hotpatch(hotpatch)?;
            (
                report,
                cartridge.payload().to_vec(),
                cartridge.mapping_info(),
            )
        };
        for image in self
            .images
            .iter_mut()
            .filter(|image| image.kind == ImageKind::Cartridge)
        {
            image.bytes = payload.clone();
            image.metadata = ImageMetadata {
                size: payload.len(),
                base: mapping.window_start,
                end: mapping.window_end,
                checksum16: checksum16(&payload),
                crc32: crc32(&payload),
            };
            image.cartridge_mapping = Some(mapping);
        }
        Ok(report)
    }

    fn load_image(&mut self, kind: ImageKind, path: PathBuf, base: u16) -> Result<(), String> {
        let bytes = fs::read(&path)
            .map_err(|err| format!("failed to read image `{}`: {err}", path.display()))?;
        let image = LoadedImage::prepare(kind, path, base, bytes)?;
        match image.kind {
            ImageKind::Ram => self.bus.ram_mut().map(base, &image.bytes)?,
            ImageKind::Rom => self.bus.map_os_rom(base, image.bytes.clone())?,
            ImageKind::Cartridge => self
                .bus
                .install_cartridge(Cartridge::from_loaded_image(&image)?),
        }
        self.images.push(image);
        Ok(())
    }
}

pub fn decode_action_symbol_tables(bus: &Bus) -> ActionSymbolTableDump {
    decode_action_symbol_tables_from_memory(bus.ram())
}

pub fn decode_action_symbol_tables_from_memory(memory: &Memory) -> ActionSymbolTableDump {
    let global_index = symbol_index_root(memory, ACTION_GLOBAL_SYMBOL_TABLE_POINTER);
    let local_index = symbol_index_root(memory, ACTION_LOCAL_SYMBOL_TABLE_POINTER);
    ActionSymbolTableDump {
        global_index,
        local_index,
        globals: global_index
            .map(|index| decode_action_symbol_table(memory, index, ActionSymbolScope::Global))
            .unwrap_or_default(),
        locals: local_index
            .map(|index| decode_action_symbol_table(memory, index, ActionSymbolScope::Local))
            .unwrap_or_default(),
    }
}

pub fn format_action_symbol_dump_json(dump: &ActionSymbolTableDump) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"global_index\": {},\n",
        format_json_optional_address(dump.global_index)
    ));
    out.push_str(&format!(
        "  \"local_index\": {},\n",
        format_json_optional_address(dump.local_index)
    ));
    out.push_str("  \"globals\": ");
    push_symbol_entries_json(&mut out, &dump.globals, 2);
    out.push_str(",\n");
    out.push_str("  \"locals\": ");
    push_symbol_entries_json(&mut out, &dump.locals, 2);
    out.push('\n');
    out.push_str("}\n");
    out
}

fn symbol_index_root(memory: &Memory, pointer_address: u16) -> Option<u16> {
    let root = memory.read_word(pointer_address);
    let root_end = u32::from(root) + 0x01FF;
    (root != 0 && root_end <= u32::from(u16::MAX)).then_some(root)
}

fn decode_action_symbol_table(
    memory: &Memory,
    index_root: u16,
    scope: ActionSymbolScope,
) -> Vec<ActionSymbolEntry> {
    let st_high = index_root;
    let st_low = st_high.wrapping_add(256);
    let mut entries = Vec::new();
    for slot in 0..=255u16 {
        let high = memory.read(st_high.wrapping_add(slot));
        if high == 0 {
            continue;
        }
        let low = memory.read(st_low.wrapping_add(slot));
        let name_addr = u16::from(low) | (u16::from(high) << 8);
        if let Some(entry) = decode_action_symbol_entry(memory, scope, slot as u8, name_addr) {
            entries.push(entry);
        }
    }
    entries.sort_by(|left, right| {
        left.name
            .to_ascii_uppercase()
            .cmp(&right.name.to_ascii_uppercase())
            .then(left.name_addr.cmp(&right.name_addr))
    });
    entries
}

fn decode_action_symbol_entry(
    memory: &Memory,
    scope: ActionSymbolScope,
    slot: u8,
    name_addr: u16,
) -> Option<ActionSymbolEntry> {
    let name_len = memory.read(name_addr);
    if name_len == 0 {
        return None;
    }
    let name_start = name_addr.wrapping_add(1);
    let entry_addr = name_start.wrapping_add(u16::from(name_len));
    let vtype = memory.read(entry_addr);
    if vtype == 0x88 {
        return None;
    }

    let name = decode_action_string_bytes(memory, name_start, name_len);
    let address = if vtype == 27 {
        None
    } else {
        Some(memory.read_word(entry_addr.wrapping_add(1)))
    };
    let numargs = if is_action_routine_type(vtype) {
        memory.read(entry_addr.wrapping_add(3))
    } else {
        0
    };
    let mut arg_types_raw = Vec::new();
    let mut args = Vec::new();
    for index in 0..numargs {
        let raw = memory.read(entry_addr.wrapping_add(4 + u16::from(index)));
        arg_types_raw.push(raw);
        args.push(describe_action_symbol_type(memory, entry_addr, raw | 0x80));
    }

    Some(ActionSymbolEntry {
        scope,
        slot,
        name_addr,
        name,
        vtype,
        address,
        class: describe_action_symbol_type(memory, entry_addr, vtype),
        numargs,
        arg_types_raw,
        args,
    })
}

fn describe_action_symbol_type(memory: &Memory, entry_addr: u16, vtype: u8) -> String {
    if vtype == 27 {
        return format!(
            "DEFINE `{}`",
            decode_action_string(memory, entry_addr.wrapping_add(3))
        );
    }
    if vtype == 39 {
        return "TYPE".to_string();
    }

    let mut parts = Vec::new();
    if is_action_routine_type(vtype) {
        if (vtype & 0xF7) == 0xC0 {
            parts.push("PROC".to_string());
        } else {
            let base = action_base_type(vtype);
            if base.is_empty() {
                parts.push("FUNC".to_string());
            } else {
                parts.push(format!("{base} FUNC"));
            }
        }
    } else if vtype < 128 {
        if (vtype & 7) == 0 {
            if (vtype & 8) == 8 {
                parts.push("RECORD POINTER".to_string());
            } else {
                parts.push("RECORD".to_string());
            }
        } else {
            let base = action_base_type(vtype);
            if base.is_empty() {
                parts.push("record field".to_string());
            } else {
                parts.push(format!("{base} record field"));
            }
        }
    } else {
        let base = action_base_type(vtype);
        if !base.is_empty() {
            parts.push(base.to_string());
        }
        if (vtype & 0x10) != 0 {
            parts.push("ARRAY".to_string());
        }
    }

    if parts.is_empty() {
        format!("vtype ${vtype:02X}")
    } else {
        parts.join(" ")
    }
}

fn is_action_routine_type(vtype: u8) -> bool {
    (vtype & 0x40) != 0 && (vtype & 0x10) == 0
}

fn action_base_type(vtype: u8) -> &'static str {
    match vtype & 7 {
        1 => "CHAR",
        2 => "BYTE",
        3 => "INT",
        4 => "CARD",
        _ => "",
    }
}

fn decode_action_string(memory: &Memory, address: u16) -> String {
    let len = memory.read(address);
    decode_action_string_bytes(memory, address.wrapping_add(1), len)
}

fn decode_action_string_bytes(memory: &Memory, start: u16, len: u8) -> String {
    (0..len)
        .map(|offset| {
            let byte = memory.read(start.wrapping_add(u16::from(offset)));
            match byte {
                0x20..=0x7E => byte as char,
                _ => '.',
            }
        })
        .collect()
}

fn push_symbol_entries_json(out: &mut String, entries: &[ActionSymbolEntry], indent: usize) {
    if entries.is_empty() {
        out.push_str("[]");
        return;
    }
    let pad = " ".repeat(indent);
    let item_pad = " ".repeat(indent + 2);
    out.push_str("[\n");
    for (index, entry) in entries.iter().enumerate() {
        let comma = if index + 1 == entries.len() { "" } else { "," };
        out.push_str(&format!(
            "{item_pad}{{\"scope\":\"{}\",\"slot\":\"${:02X}\",\"name_addr\":\"${:04X}\",\"name\":\"{}\",\"vtype\":\"${:02X}\",\"address\":{},\"class\":\"{}\",\"numargs\":{},\"arg_types_raw\":[{}],\"args\":[{}]}}{comma}\n",
            action_symbol_scope_name(entry.scope),
            entry.slot,
            entry.name_addr,
            escape_json(&entry.name),
            entry.vtype,
            format_json_optional_address(entry.address),
            escape_json(&entry.class),
            entry.numargs,
            format_json_byte_array(&entry.arg_types_raw),
            format_json_string_array(&entry.args),
        ));
    }
    out.push_str(&format!("{pad}]"));
}

fn action_symbol_scope_name(scope: ActionSymbolScope) -> &'static str {
    match scope {
        ActionSymbolScope::Global => "global",
        ActionSymbolScope::Local => "local",
    }
}

fn format_json_optional_address(address: Option<u16>) -> String {
    address
        .map(|address| format!("\"${address:04X}\""))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSourceInjectionReport {
    pub line_count: usize,
    pub first_line: Option<u16>,
    pub last_line: Option<u16>,
    pub allocated_bytes: u16,
    pub free_head: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionEditorLine {
    pub address: u16,
    pub previous: u16,
    pub next: u16,
    pub allocation_size: u16,
    pub length: u8,
    pub text: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSymbolTableDump {
    pub global_index: Option<u16>,
    pub local_index: Option<u16>,
    pub globals: Vec<ActionSymbolEntry>,
    pub locals: Vec<ActionSymbolEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSymbolEntry {
    pub scope: ActionSymbolScope,
    pub slot: u8,
    pub name_addr: u16,
    pub name: String,
    pub vtype: u8,
    pub address: Option<u16>,
    pub class: String,
    pub numargs: u8,
    pub arg_types_raw: Vec<u8>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSymbolScope {
    Global,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextScreenSnapshot {
    pub base: u16,
    pub columns: usize,
    pub rows: usize,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cpu {
    registers: CpuRegisters,
    cycles: u64,
    halted: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            registers: CpuRegisters::default(),
            cycles: 0,
            halted: false,
        }
    }
}

impl Cpu {
    pub fn registers(&self) -> CpuRegisters {
        self.registers
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    fn set_pc(&mut self, pc: u16) {
        self.registers.pc = pc;
    }

    pub fn reset(&mut self, bus: &mut Bus) {
        let lo = bus.read(RESET_VECTOR);
        let hi = bus.read(RESET_VECTOR.wrapping_add(1));
        self.registers = CpuRegisters {
            pc: u16::from_le_bytes([lo, hi]),
            sp: 0xFD,
            status: StatusFlags::INTERRUPT_DISABLE.bits() | StatusFlags::UNUSED.bits(),
            ..CpuRegisters::default()
        };
        self.cycles = 7;
        self.halted = false;
    }

    pub fn step(&mut self, bus: &mut Bus) -> Result<CpuStep, CpuError> {
        if self.halted {
            return Err(CpuError::Halted);
        }

        let pc = self.registers.pc;
        let registers_before = self.registers;
        if pc == CIOV && self.try_emulate_ciov(bus) {
            return Ok(CpuStep {
                pc,
                opcode: 0xFF,
                registers_before,
                registers_after: self.registers,
                cycles: self.cycles,
            });
        }

        let opcode = self.fetch_byte(bus);

        match opcode {
            0x01 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 6;
            }
            0x05 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 3;
            }
            0x06 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = value << 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 5;
            }
            0x08 => {
                self.push(
                    bus,
                    self.registers.status | StatusFlags::UNUSED.bits() | 0x10,
                );
                self.cycles += 3;
            }
            0x09 => {
                let value = self.fetch_byte(bus);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x0A => {
                let value = self.registers.a;
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                self.registers.a = value << 1;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x0D => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x0E => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = value << 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
            }
            0x10 => {
                self.branch(bus, !self.flag(StatusFlags::NEGATIVE), 2, 3);
            }
            0x11 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 5;
            }
            0x15 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x18 => {
                self.set_flag(StatusFlags::CARRY, false);
                self.cycles += 2;
            }
            0x19 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x1D => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.registers.a |= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x20 => {
                let target = self.fetch_word(bus);
                let return_address = self.registers.pc.wrapping_sub(1);
                self.push(bus, (return_address >> 8) as u8);
                self.push(bus, return_address as u8);
                self.registers.pc = target;
                self.cycles += 6;
            }
            0x26 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                let carry_in = u8::from(self.flag(StatusFlags::CARRY));
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = (value << 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 5;
            }
            0x25 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 3;
            }
            0x28 => {
                self.registers.status = self.pop(bus) | StatusFlags::UNUSED.bits();
                self.cycles += 4;
            }
            0x29 => {
                let value = self.fetch_byte(bus);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x2A => {
                let value = self.registers.a;
                let carry_in = u8::from(self.flag(StatusFlags::CARRY));
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                self.registers.a = (value << 1) | carry_in;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x2C => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.set_flag(StatusFlags::ZERO, self.registers.a & value == 0);
                self.set_flag(StatusFlags::NEGATIVE, value & 0x80 != 0);
                self.set_flag(StatusFlags::OVERFLOW, value & 0x40 != 0);
                self.cycles += 4;
            }
            0x2D => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x2E => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                let carry_in = u8::from(self.flag(StatusFlags::CARRY));
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = (value << 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
            }
            0x30 => {
                self.branch(bus, self.flag(StatusFlags::NEGATIVE), 2, 3);
            }
            0x36 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                let carry_in = u8::from(self.flag(StatusFlags::CARRY));
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = (value << 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
            }
            0x31 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 5;
            }
            0x38 => {
                self.set_flag(StatusFlags::CARRY, true);
                self.cycles += 2;
            }
            0x3D => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x45 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 3;
            }
            0x46 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = value >> 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 5;
            }
            0x48 => {
                self.push(bus, self.registers.a);
                self.cycles += 3;
            }
            0x49 => {
                let value = self.fetch_byte(bus);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x4A => {
                let value = self.registers.a;
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                self.registers.a = value >> 1;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x4C => {
                let target = self.fetch_word(bus);
                self.registers.pc = target;
                self.cycles += 3;
            }
            0x4D => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x4E => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = value >> 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
            }
            0x51 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 5;
            }
            0x58 => {
                self.set_flag(StatusFlags::INTERRUPT_DISABLE, false);
                self.cycles += 2;
            }
            0x60 => {
                let lo = self.pop(bus);
                let hi = self.pop(bus);
                self.registers.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
                self.cycles += 6;
            }
            0x65 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 3;
            }
            0x66 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                let carry_in = if self.flag(StatusFlags::CARRY) {
                    0x80
                } else {
                    0x00
                };
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = (value >> 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 5;
            }
            0x68 => {
                self.registers.a = self.pop(bus);
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x69 => {
                let value = self.fetch_byte(bus);
                self.adc(value);
                self.cycles += 2;
            }
            0x6D => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 4;
            }
            0x6A => {
                let carry_in = if self.flag(StatusFlags::CARRY) {
                    0x80
                } else {
                    0x00
                };
                let old = self.registers.a;
                self.set_flag(StatusFlags::CARRY, old & 0x01 != 0);
                self.registers.a = (old >> 1) | carry_in;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x6C => {
                let pointer = self.fetch_word(bus);
                let target = self.read_indirect_6502_bug(bus, pointer);
                self.registers.pc = target;
                self.cycles += 5;
            }
            0x6E => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                let carry_in = if self.flag(StatusFlags::CARRY) {
                    0x80
                } else {
                    0x00
                };
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = (value >> 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
            }
            0x71 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 5;
            }
            0x75 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 4;
            }
            0x78 => {
                self.set_flag(StatusFlags::INTERRUPT_DISABLE, true);
                self.cycles += 2;
            }
            0x7D => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 4;
            }
            0x7E => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                let carry_in = if self.flag(StatusFlags::CARRY) {
                    0x80
                } else {
                    0x00
                };
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = (value >> 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 7;
            }
            0x84 => {
                let address = self.fetch_byte(bus) as u16;
                bus.write(address, self.registers.y);
                self.cycles += 3;
            }
            0x85 => {
                let address = self.fetch_byte(bus) as u16;
                bus.write(address, self.registers.a);
                self.cycles += 3;
            }
            0x86 => {
                let address = self.fetch_byte(bus) as u16;
                bus.write(address, self.registers.x);
                self.cycles += 3;
            }
            0x88 => {
                self.registers.y = self.registers.y.wrapping_sub(1);
                self.set_zn(self.registers.y);
                self.cycles += 2;
            }
            0x8A => {
                self.registers.a = self.registers.x;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x8C => {
                let address = self.fetch_word(bus);
                bus.write(address, self.registers.y);
                self.cycles += 4;
            }
            0x8D => {
                let address = self.fetch_word(bus);
                bus.write(address, self.registers.a);
                self.cycles += 4;
            }
            0x8E => {
                let address = self.fetch_word(bus);
                bus.write(address, self.registers.x);
                self.cycles += 4;
            }
            0x90 => {
                self.branch(bus, !self.flag(StatusFlags::CARRY), 2, 3);
            }
            0x91 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                bus.write(address, self.registers.a);
                self.cycles += 6;
            }
            0x95 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                bus.write(address, self.registers.a);
                self.cycles += 4;
            }
            0x98 => {
                self.registers.a = self.registers.y;
                self.set_zn(self.registers.a);
                self.cycles += 2;
            }
            0x99 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                bus.write(address, self.registers.a);
                self.cycles += 5;
            }
            0x9A => {
                self.registers.sp = self.registers.x;
                self.cycles += 2;
            }
            0x9D => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                bus.write(address, self.registers.a);
                self.cycles += 5;
            }
            0xA0 => {
                let value = self.fetch_byte(bus);
                self.registers.y = value;
                self.set_zn(value);
                self.cycles += 2;
            }
            0xA2 => {
                let value = self.fetch_byte(bus);
                self.registers.x = value;
                self.set_zn(value);
                self.cycles += 2;
            }
            0xA4 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.registers.y = value;
                self.set_zn(value);
                self.cycles += 3;
            }
            0xA5 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 3;
            }
            0xA6 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.registers.x = value;
                self.set_zn(value);
                self.cycles += 3;
            }
            0xA8 => {
                self.registers.y = self.registers.a;
                self.set_zn(self.registers.y);
                self.cycles += 2;
            }
            0xA9 => {
                let value = self.fetch_byte(bus);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 2;
            }
            0xAA => {
                self.registers.x = self.registers.a;
                self.set_zn(self.registers.x);
                self.cycles += 2;
            }
            0xAC => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.registers.y = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xAD => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xAE => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.registers.x = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xB0 => {
                self.branch(bus, self.flag(StatusFlags::CARRY), 2, 3);
            }
            0xB8 => {
                self.set_flag(StatusFlags::OVERFLOW, false);
                self.cycles += 2;
            }
            0xB1 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 5;
            }
            0xB5 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xB9 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xBA => {
                self.registers.x = self.registers.sp;
                self.set_zn(self.registers.x);
                self.cycles += 2;
            }
            0xBC => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.registers.y = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xBD => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xBE => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.registers.x = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xC0 => {
                let value = self.fetch_byte(bus);
                self.compare(self.registers.y, value);
                self.cycles += 2;
            }
            0xC4 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.compare(self.registers.y, value);
                self.cycles += 3;
            }
            0xC5 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 3;
            }
            0xC6 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address).wrapping_sub(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 5;
            }
            0xC8 => {
                self.registers.y = self.registers.y.wrapping_add(1);
                self.set_zn(self.registers.y);
                self.cycles += 2;
            }
            0xC9 => {
                let value = self.fetch_byte(bus);
                self.compare(self.registers.a, value);
                self.cycles += 2;
            }
            0xCA => {
                self.registers.x = self.registers.x.wrapping_sub(1);
                self.set_zn(self.registers.x);
                self.cycles += 2;
            }
            0xCC => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.compare(self.registers.y, value);
                self.cycles += 4;
            }
            0xCD => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 4;
            }
            0xCE => {
                let address = self.fetch_word(bus);
                let value = bus.read(address).wrapping_sub(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 6;
            }
            0xD0 => {
                self.branch(bus, !self.flag(StatusFlags::ZERO), 2, 3);
            }
            0xD1 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 5;
            }
            0xD8 => {
                self.set_flag(StatusFlags::DECIMAL, false);
                self.cycles += 2;
            }
            0xD9 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 4;
            }
            0xDE => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address).wrapping_sub(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 7;
            }
            0xDD => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 4;
            }
            0xE0 => {
                let value = self.fetch_byte(bus);
                self.compare(self.registers.x, value);
                self.cycles += 2;
            }
            0xE1 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 6;
            }
            0xE4 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.compare(self.registers.x, value);
                self.cycles += 3;
            }
            0xE5 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 3;
            }
            0xE6 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address).wrapping_add(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 5;
            }
            0xE8 => {
                self.registers.x = self.registers.x.wrapping_add(1);
                self.set_zn(self.registers.x);
                self.cycles += 2;
            }
            0xE9 => {
                let value = self.fetch_byte(bus);
                self.sbc(value);
                self.cycles += 2;
            }
            0xEA => {
                self.cycles += 2;
            }
            0xED => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 4;
            }
            0xEE => {
                let address = self.fetch_word(bus);
                let value = bus.read(address).wrapping_add(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 6;
            }
            0xF0 => {
                self.branch(bus, self.flag(StatusFlags::ZERO), 2, 3);
            }
            0xF1 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 5;
            }
            0xF5 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 4;
            }
            0xF8 => {
                self.set_flag(StatusFlags::DECIMAL, true);
                self.cycles += 2;
            }
            0xF9 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 4;
            }
            0xFD => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.sbc(value);
                self.cycles += 4;
            }
            0xFE => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address).wrapping_add(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 7;
            }
            opcode => {
                self.halted = true;
                return Err(CpuError::UnsupportedOpcode { pc, opcode });
            }
        }

        Ok(CpuStep {
            pc,
            opcode,
            registers_before,
            registers_after: self.registers,
            cycles: self.cycles,
        })
    }

    fn fetch_byte(&mut self, bus: &mut Bus) -> u8 {
        let value = bus.read(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    fn fetch_word(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.fetch_byte(bus);
        let hi = self.fetch_byte(bus);
        u16::from_le_bytes([lo, hi])
    }

    fn read_word(&mut self, bus: &mut Bus, address: u16) -> u16 {
        let lo = bus.read(address);
        let hi = bus.read(address.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    fn read_indirect_6502_bug(&mut self, bus: &mut Bus, address: u16) -> u16 {
        let lo = bus.read(address);
        let hi_address = (address & 0xFF00) | address.wrapping_add(1) & 0x00FF;
        let hi = bus.read(hi_address);
        u16::from_le_bytes([lo, hi])
    }

    fn indirect_y(&mut self, bus: &mut Bus, zp: u8) -> u16 {
        let base = self.read_word(bus, zp as u16);
        base.wrapping_add(self.registers.y as u16)
    }

    fn indexed_indirect(&mut self, bus: &mut Bus, zp: u8) -> u16 {
        let pointer = zp.wrapping_add(self.registers.x);
        self.read_word(bus, pointer as u16)
    }

    fn push(&mut self, bus: &mut Bus, value: u8) {
        let address = 0x0100 | self.registers.sp as u16;
        bus.write(address, value);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
    }

    fn pop(&mut self, bus: &mut Bus) -> u8 {
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let address = 0x0100 | self.registers.sp as u16;
        bus.read(address)
    }

    fn try_emulate_ciov(&mut self, bus: &mut Bus) -> bool {
        let command_address = IOCB_COMMAND_BASE.wrapping_add(self.registers.x as u16);
        let command = bus.ram().read(command_address);
        let return_pc = self.peek_return_address(bus);
        bus.trace_cio_call(self.registers.x, command, return_pc);
        match command {
            CIO_COMMAND_OPEN => {
                if bus.try_open_harness_cio_device(self.registers.x) {
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                false
            }
            CIO_COMMAND_GETCHR | CIO_COMMAND_GETREC => {
                match bus.cio_channel_device(self.registers.x) {
                    Some(CioHarnessDevice::QueuedInput) => {
                        if let Some(character) = bus.pop_scripted_cio_input_byte() {
                            bus.trace_cio(format_args!(
                                "  Q: read ${character:02X} `{}`",
                                atari_debug_char(character)
                            ));
                            self.return_from_ciov(bus, character, 0x01);
                            return true;
                        }
                    }
                    Some(CioHarnessDevice::Host { .. }) => {
                        let result = if command == CIO_COMMAND_GETREC {
                            bus.read_host_record(self.registers.x)
                        } else {
                            bus.read_host_character(self.registers.x)
                        };
                        if let Some((accumulator, status)) = result {
                            self.return_from_ciov(bus, accumulator, status);
                            return true;
                        }
                    }
                    None => {}
                }

                if self.registers.x != 0x70 {
                    return false;
                }
                let raw_key = bus.ram().read(CH_KEY_CODE);
                let Some(character) = atari_key_code_to_character(raw_key) else {
                    return false;
                };

                bus.write(CH_KEY_CODE, 0xFF);
                self.return_from_ciov(bus, character, 0x01);
                true
            }
            CIO_COMMAND_CLOSE => {
                if bus.close_harness_cio_device(self.registers.x) {
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                false
            }
            CIO_COMMAND_STATUS => {
                if bus.cio_channel_device(self.registers.x).is_some() {
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                false
            }
            CIO_COMMAND_PUTCHR | CIO_COMMAND_PUTREC => {
                if bus
                    .write_host_bytes_for_iocb(self.registers.x, self.registers.a)
                    .is_some()
                {
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                if self.registers.x != 0x00 {
                    return false;
                }
                let bytes = bus.cio_output_bytes_for_iocb(self.registers.x, self.registers.a);
                bus.capture_cio_channel0_output(&bytes);
                self.return_from_ciov(bus, self.registers.a, 0x01);
                true
            }
            _ => false,
        }
    }

    fn return_from_ciov(&mut self, bus: &mut Bus, a: u8, y: u8) {
        self.registers.a = a;
        self.registers.y = y;
        let lo = self.pop(bus);
        let hi = self.pop(bus);
        self.registers.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
        self.set_zn(self.registers.y);
        self.cycles += 6;
    }

    fn peek_return_address(&self, bus: &Bus) -> u16 {
        let lo_address = 0x0100 | self.registers.sp.wrapping_add(1) as u16;
        let hi_address = 0x0100 | self.registers.sp.wrapping_add(2) as u16;
        let lo = bus.ram().read(lo_address);
        let hi = bus.ram().read(hi_address);
        u16::from_le_bytes([lo, hi]).wrapping_add(1)
    }

    fn branch(&mut self, bus: &mut Bus, condition: bool, base_cycles: u64, branch_cycles: u64) {
        let offset = self.fetch_byte(bus) as i8;
        if condition {
            self.registers.pc = self.registers.pc.wrapping_add_signed(offset as i16);
            self.cycles += branch_cycles;
        } else {
            self.cycles += base_cycles;
        }
    }

    fn compare(&mut self, register: u8, value: u8) {
        let result = register.wrapping_sub(value);
        self.set_flag(StatusFlags::CARRY, register >= value);
        self.set_zn(result);
    }

    fn adc(&mut self, value: u8) {
        let carry = u8::from(self.flag(StatusFlags::CARRY));
        let lhs = self.registers.a;
        let sum = lhs as u16 + value as u16 + carry as u16;
        let result = sum as u8;
        self.set_flag(StatusFlags::CARRY, sum > 0xFF);
        self.set_flag(
            StatusFlags::OVERFLOW,
            (lhs ^ result) & (value ^ result) & 0x80 != 0,
        );
        self.registers.a = result;
        self.set_zn(result);
    }

    fn sbc(&mut self, value: u8) {
        let borrow = u8::from(!self.flag(StatusFlags::CARRY));
        let lhs = self.registers.a;
        let result = lhs.wrapping_sub(value).wrapping_sub(borrow);
        self.set_flag(
            StatusFlags::CARRY,
            (lhs as u16) >= (value as u16 + borrow as u16),
        );
        self.set_flag(
            StatusFlags::OVERFLOW,
            (lhs ^ result) & (lhs ^ value) & 0x80 != 0,
        );
        self.registers.a = result;
        self.set_zn(result);
    }

    fn set_zn(&mut self, value: u8) {
        self.set_flag(StatusFlags::ZERO, value == 0);
        self.set_flag(StatusFlags::NEGATIVE, value & 0x80 != 0);
    }

    fn flag(&self, flag: StatusFlags) -> bool {
        self.registers.status & flag.bits() != 0
    }

    fn set_flag(&mut self, flag: StatusFlags, enabled: bool) {
        if enabled {
            self.registers.status |= flag.bits();
        } else {
            self.registers.status &= !flag.bits();
        }
        self.registers.status |= StatusFlags::UNUSED.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuRegisters {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
}

impl Default for CpuRegisters {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: StatusFlags::UNUSED.bits(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuStep {
    pub pc: u16,
    pub opcode: u8,
    pub registers_before: CpuRegisters,
    pub registers_after: CpuRegisters,
    pub cycles: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuError {
    Halted,
    UnsupportedOpcode { pc: u16, opcode: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusFlags(u8);

impl StatusFlags {
    const CARRY: Self = Self(0x01);
    const ZERO: Self = Self(0x02);
    const INTERRUPT_DISABLE: Self = Self(0x04);
    const DECIMAL: Self = Self(0x08);
    const UNUSED: Self = Self(0x20);
    const OVERFLOW: Self = Self(0x40);
    const NEGATIVE: Self = Self(0x80);

    const fn bits(self) -> u8 {
        self.0
    }
}

impl LoadedImage {
    fn prepare(kind: ImageKind, path: PathBuf, base: u16, bytes: Vec<u8>) -> Result<Self, String> {
        match kind {
            ImageKind::Cartridge => Self::prepare_cartridge(path, base, bytes),
            ImageKind::Ram | ImageKind::Rom => {
                let metadata = ImageMetadata::from_bytes(base, &bytes)
                    .map_err(|err| format!("invalid image `{}`: {err}", path.display()))?;
                Ok(Self {
                    kind,
                    path,
                    base,
                    metadata,
                    car_header: None,
                    cartridge_mapping: None,
                    bytes,
                })
            }
        }
    }

    fn prepare_cartridge(path: PathBuf, base: u16, bytes: Vec<u8>) -> Result<Self, String> {
        let (car_header, payload) = parse_car_container(&bytes);
        let payload = payload.to_vec();
        let cartridge = Cartridge::from_payload(base, car_header, payload.clone())
            .map_err(|err| format!("invalid cartridge `{}`: {err}", path.display()))?;
        let mapping = cartridge.mapping_info();
        let metadata = ImageMetadata {
            size: payload.len(),
            base: mapping.window_start,
            end: mapping.window_end,
            checksum16: checksum16(&payload),
            crc32: crc32(&payload),
        };

        Ok(Self {
            kind: ImageKind::Cartridge,
            path,
            base,
            metadata,
            car_header,
            cartridge_mapping: Some(mapping),
            bytes: payload,
        })
    }
}

fn parse_car_container(bytes: &[u8]) -> (Option<CarHeader>, &[u8]) {
    if bytes.len() <= CAR_HEADER_SIZE || &bytes[..4] != CAR_MAGIC {
        return (None, bytes);
    }

    let cartridge_type = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let checksum = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    (
        Some(CarHeader {
            cartridge_type,
            checksum,
        }),
        &bytes[CAR_HEADER_SIZE..],
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bus {
    ram: Memory,
    io: IoRegion,
    os_rom: Option<RomRegion>,
    cartridge: Option<Cartridge>,
    watchpoints: Vec<AddressRange>,
    events: Vec<BusEvent>,
    last_data: u8,
    vcount: u8,
    pending_key_codes: VecDeque<u8>,
    scripted_cio_input: VecDeque<u8>,
    cio_channel0_output: Vec<u8>,
    cio_harness_devices: [Option<CioHarnessDevice>; 8],
    host_files: Vec<HostFile>,
    host_file_lookup: HashMap<String, usize>,
    trace_cio: bool,
    sio_timeout_pending: bool,
    redirect_disk_boot_to_cart: bool,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            ram: Memory::default(),
            io: IoRegion::default(),
            os_rom: None,
            cartridge: None,
            watchpoints: Vec::new(),
            events: Vec::new(),
            last_data: 0,
            vcount: 0,
            pending_key_codes: VecDeque::new(),
            scripted_cio_input: VecDeque::new(),
            cio_channel0_output: Vec::new(),
            cio_harness_devices: [None; 8],
            host_files: Vec::new(),
            host_file_lookup: HashMap::new(),
            trace_cio: false,
            sio_timeout_pending: false,
            redirect_disk_boot_to_cart: false,
        }
    }
}

impl Bus {
    pub fn ram(&self) -> &Memory {
        &self.ram
    }

    pub fn ram_mut(&mut self) -> &mut Memory {
        &mut self.ram
    }

    pub fn cartridge(&self) -> Option<&Cartridge> {
        self.cartridge.as_ref()
    }

    pub fn cartridge_mut(&mut self) -> Option<&mut Cartridge> {
        self.cartridge.as_mut()
    }

    pub fn os_rom(&self) -> Option<&RomRegion> {
        self.os_rom.as_ref()
    }

    pub fn io(&self) -> &IoRegion {
        &self.io
    }

    pub fn add_watchpoint(&mut self, address: u16) {
        self.add_watch_range(AddressRange {
            start: address,
            end: address,
        });
    }

    pub fn add_watch_range(&mut self, range: AddressRange) {
        if !self.watchpoints.contains(&range) {
            self.watchpoints.push(range);
        }
    }

    pub fn events(&self) -> &[BusEvent] {
        &self.events
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn queue_key_code(&mut self, key_code: u8) {
        if self.ram.read(CH_KEY_CODE) == 0xFF {
            self.deliver_key_code(key_code);
        } else {
            self.pending_key_codes.push_back(key_code);
        }
    }

    pub fn queue_scripted_cio_input_byte(&mut self, byte: u8) {
        self.scripted_cio_input.push_back(byte);
    }

    pub fn queue_scripted_cio_input_bytes(&mut self, bytes: &[u8]) {
        self.scripted_cio_input.extend(bytes);
    }

    pub fn add_host_file(&mut self, name: impl AsRef<str>, bytes: Vec<u8>) {
        let normalized = normalize_host_file_name(name.as_ref());
        let index = self.host_files.len();
        self.host_files.push(HostFile {
            name: normalized.clone(),
            bytes,
            writable: false,
        });
        self.host_file_lookup.insert(normalized, index);
    }

    pub fn add_host_output(&mut self, name: impl AsRef<str>) {
        let normalized = normalize_host_file_name(name.as_ref());
        let index = self.host_files.len();
        self.host_files.push(HostFile {
            name: normalized.clone(),
            bytes: Vec::new(),
            writable: true,
        });
        self.host_file_lookup.insert(normalized, index);
    }

    pub fn host_file_bytes(&self, name: impl AsRef<str>) -> Option<&[u8]> {
        let normalized = normalize_host_file_name(name.as_ref());
        let index = self.host_file_lookup.get(&normalized)?;
        Some(&self.host_files[*index].bytes)
    }

    pub fn set_trace_cio(&mut self, trace_cio: bool) {
        self.trace_cio = trace_cio;
    }

    pub fn cio_channel0_output(&self) -> &[u8] {
        &self.cio_channel0_output
    }

    pub fn decoded_cio_channel0_output(&self) -> String {
        decode_atascii_output(&self.cio_channel0_output)
    }

    pub fn inject_action_source(
        &mut self,
        source: &[u8],
    ) -> Result<ActionSourceInjectionReport, String> {
        let lines = split_action_source_lines(source);
        let line_max = self.action_line_max();
        for line in &lines {
            if line.len() > line_max as usize {
                return Err(format!(
                    "source line is {} byte(s), exceeding Action! line limit {line_max}",
                    line.len()
                ));
            }
        }

        let buf = self.ram.read_word(ACTION_BUF);
        if buf == 0 {
            return Err(
                "Action! edit buffer pointer is zero; editor is not initialized".to_string(),
            );
        }

        let old_top = self.ram.read_word(ACTION_TOP);
        if old_top != 0 {
            self.free_action_line_list(old_top)?;
        }

        let mut records = Vec::new();
        let mut allocated_bytes = 0u16;
        for &line in &lines {
            let allocation_size = ACTION_LINE_ALLOC_OVERHEAD
                .checked_add(line.len() as u16)
                .ok_or_else(|| "source line allocation size overflowed".to_string())?;
            let address = self.allocate_action_heap(allocation_size)?;
            allocated_bytes = allocated_bytes
                .checked_add(allocation_size)
                .ok_or_else(|| "source allocation total overflowed".to_string())?;
            records.push((address, allocation_size, line));
        }

        for index in 0..records.len() {
            let (address, allocation_size, line) = records[index];
            let previous = if index == 0 { 0 } else { records[index - 1].0 };
            let next = if index + 1 == records.len() {
                0
            } else {
                records[index + 1].0
            };

            self.ram.write_word(address, previous);
            self.ram
                .write_word(address.wrapping_add(2), allocation_size);
            self.ram.write_word(address.wrapping_add(4), next);
            self.ram.write(
                address.wrapping_add(ACTION_LINE_HEADER_SIZE),
                line.len() as u8,
            );
            if !line.is_empty() {
                self.ram
                    .map(address.wrapping_add(ACTION_LINE_HEADER_SIZE + 1), line)?;
            }
        }

        let first_line = records.first().map(|record| record.0);
        let last_line = records.last().map(|record| record.0);
        let first = first_line.unwrap_or(0);
        let last = last_line.unwrap_or(0);

        self.ram.write_word(ACTION_TOP, first);
        self.ram.write_word(ACTION_BOT, last);
        self.ram.write_word(ACTION_CUR, first);
        self.ram.write(ACTION_DIRTY, 0);
        self.ram.write(ACTION_DIRTYF, 0);
        self.ram.write(ACTION_CHOFF, 0);
        self.ram.write(ACTION_LNUM, 0);
        self.ram
            .write_word(ACTION_VARS_W1.wrapping_add(ACTION_WINDOW_TOP_OFFSET), first);
        self.ram
            .write_word(ACTION_VARS_W1.wrapping_add(ACTION_WINDOW_BOT_OFFSET), last);
        self.ram
            .write_word(ACTION_VARS_W1.wrapping_add(ACTION_WINDOW_CUR_OFFSET), first);
        self.ram.write(ACTION_VARS_W1.wrapping_add(3), 0);
        self.ram.write(ACTION_VARS_TOP1, (first >> 8) as u8);

        if let Some((_, _, first_text)) = records.first() {
            self.write_action_scratch_line(first_text)?;
        } else {
            self.ram.write(buf, 0);
        }

        Ok(ActionSourceInjectionReport {
            line_count: records.len(),
            first_line,
            last_line,
            allocated_bytes,
            free_head: self.ram.read_word(ACTION_AFBASE),
        })
    }

    pub fn action_editor_lines(&self) -> Result<Vec<ActionEditorLine>, String> {
        let mut lines = Vec::new();
        let mut address = self.ram.read_word(ACTION_TOP);
        let mut previous = 0;

        for _ in 0..1024 {
            if address == 0 {
                return Ok(lines);
            }

            let line_previous = self.ram.read_word(address);
            let allocation_size = self.ram.read_word(address.wrapping_add(2));
            let next = self.ram.read_word(address.wrapping_add(4));
            let length = self.ram.read(address.wrapping_add(ACTION_LINE_HEADER_SIZE));
            if allocation_size < ACTION_LINE_ALLOC_OVERHEAD {
                return Err(format!(
                    "line at ${address:04X} has invalid allocation size {allocation_size}"
                ));
            }
            if length as u16 > allocation_size - ACTION_LINE_ALLOC_OVERHEAD {
                return Err(format!(
                    "line at ${address:04X} length {length} exceeds allocation payload"
                ));
            }
            if line_previous != previous {
                return Err(format!(
                    "line at ${address:04X} has prev ${line_previous:04X}, expected ${previous:04X}"
                ));
            }

            let text_start = address.wrapping_add(ACTION_LINE_HEADER_SIZE + 1);
            let mut text = Vec::with_capacity(length as usize);
            for offset in 0..length as u16 {
                text.push(self.ram.read(text_start.wrapping_add(offset)));
            }

            lines.push(ActionEditorLine {
                address,
                previous: line_previous,
                next,
                allocation_size,
                length,
                text,
            });

            previous = address;
            address = next;
        }

        Err("Action! editor line list did not terminate within 1024 lines".to_string())
    }

    pub fn text_screen_snapshot(&self, columns: usize, rows: usize) -> TextScreenSnapshot {
        let base = self.text_screen_base();
        let mut lines = Vec::with_capacity(rows);
        for row in 0..rows {
            let mut line = String::with_capacity(columns);
            for column in 0..columns {
                let offset = row
                    .checked_mul(columns)
                    .and_then(|offset| offset.checked_add(column))
                    .unwrap_or(usize::MAX);
                let value = if offset <= u16::MAX as usize {
                    self.ram.read(base.wrapping_add(offset as u16))
                } else {
                    0
                };
                line.push(atari_screen_code_to_ascii(value));
            }
            lines.push(line);
        }
        TextScreenSnapshot {
            base,
            columns,
            rows,
            lines,
        }
    }

    pub fn visible_action_error(&self) -> Option<String> {
        let snapshot = self.text_screen_snapshot(40, 24);
        if let Some(line) = snapshot
            .lines
            .iter()
            .map(|line| line.trim_end().to_string())
            .find(|line| line.to_ascii_lowercase().contains("error:"))
        {
            return Some(line);
        }
        self.decoded_ram_line_containing_action_error()
    }

    pub fn speaker_write_count(&self) -> u64 {
        self.io.speaker_write_count()
    }

    pub fn last_speaker_write(&self) -> Option<u8> {
        self.io.last_speaker_write()
    }

    pub fn text_screen_base(&self) -> u16 {
        self.display_list_screen_base()
            .or_else(|| ram_address(self.ram.read_word(SAVMSC_SCREEN_MEMORY_POINTER)))
            .unwrap_or_else(|| self.ram.read_word(SAVMSC_SCREEN_MEMORY_POINTER))
    }

    pub fn map_os_rom(&mut self, base: u16, bytes: Vec<u8>) -> Result<(), String> {
        self.os_rom = Some(RomRegion::new(base, bytes)?);
        Ok(())
    }

    pub fn install_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    pub fn read(&mut self, address: u16) -> u8 {
        let (mut value, region) = if let Some(cartridge) = self.cartridge.as_mut() {
            if cartridge.control_access(address) {
                (self.last_data, BusRegion::CartridgeControl)
            } else if let Some(value) = cartridge.read(address) {
                (value, BusRegion::Cartridge)
            } else if let Some(value) = self.read_io(address) {
                (value, BusRegion::Io)
            } else if let Some(value) = self.read_self_test(address) {
                (value, BusRegion::SelfTestRom)
            } else if let Some(os_rom) = self.os_rom.as_ref() {
                if let Some(value) = os_rom.read(address) {
                    (value, BusRegion::OsRom)
                } else {
                    (self.read_ram(address), BusRegion::Ram)
                }
            } else {
                (self.read_ram(address), BusRegion::Ram)
            }
        } else if let Some(value) = self.read_io(address) {
            (value, BusRegion::Io)
        } else if let Some(value) = self.read_self_test(address) {
            (value, BusRegion::SelfTestRom)
        } else if let Some(os_rom) = self.os_rom.as_ref() {
            if let Some(value) = os_rom.read(address) {
                (value, BusRegion::OsRom)
            } else {
                (self.read_ram(address), BusRegion::Ram)
            }
        } else {
            (self.read_ram(address), BusRegion::Ram)
        };

        if address == TIMFLG_TIMEOUT_FLAG && self.sio_timeout_pending {
            value = 0x00;
            self.sio_timeout_pending = false;
            self.ram.write(TIMFLG_TIMEOUT_FLAG, value);
            self.redirect_disk_boot_to_cart = true;
            self.point_dosvec_to_cartridge_coldstart();
        }

        self.last_data = value;
        self.record_event(BusAccess::Read, address, value, region);
        value
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let region = if let Some(cartridge) = self.cartridge.as_mut() {
            if cartridge.write(address, value) {
                BusRegion::CartridgeControl
            } else if cartridge.contains(address) {
                BusRegion::Cartridge
            } else if self.io.write(address, value) {
                BusRegion::Io
            } else if self
                .os_rom
                .as_ref()
                .is_some_and(|os_rom| os_rom.contains(address))
            {
                BusRegion::OsRom
            } else {
                self.ram.write(address, value);
                BusRegion::Ram
            }
        } else if self.io.write(address, value) {
            BusRegion::Io
        } else if self
            .os_rom
            .as_ref()
            .is_some_and(|os_rom| os_rom.contains(address))
        {
            BusRegion::OsRom
        } else {
            self.ram.write(address, value);
            BusRegion::Ram
        };

        if address == SEROUT_SERIAL_OUTPUT {
            self.ram.write(XMTDON_TRANSMISSION_DONE_FLAG, 0xFF);
            self.ram.write(RECVDN_RECEIVE_DONE_FLAG, 0x00);
            self.sio_timeout_pending = true;
        }
        if self.redirect_disk_boot_to_cart
            && (address == BOOTQ_SUCCESSFUL_BOOT_FLAG
                || address == DOSVEC_START_VECTOR
                || address == DOSVEC_START_VECTOR.wrapping_add(1))
        {
            self.point_dosvec_to_cartridge_coldstart();
        }

        self.last_data = value;
        self.record_event(BusAccess::Write, address, value, region);
    }

    fn record_event(&mut self, access: BusAccess, address: u16, value: u8, region: BusRegion) {
        if self.watchpoints.iter().any(|range| range.contains(address)) {
            self.events.push(BusEvent {
                access,
                address,
                value,
                region,
            });
        }
    }

    fn point_dosvec_to_cartridge_coldstart(&mut self) {
        let Some(target) = self.cartridge_word(CARTCS_COLDSTART_VECTOR) else {
            return;
        };

        self.apply_headless_memory_defaults();
        let [lo, hi] = target.to_le_bytes();
        self.ram.write(BOOTQ_SUCCESSFUL_BOOT_FLAG, 0x01);
        self.ram.write(DOSVEC_START_VECTOR, lo);
        self.ram.write(DOSVEC_START_VECTOR.wrapping_add(1), hi);
    }

    fn apply_headless_memory_defaults(&mut self) {
        self.ram
            .write(RAMTOP_MEMORY_TOP_PAGE, DEFAULT_HEADLESS_RAMTOP_PAGE);
        self.ram
            .write_word(MEMTOP_OS_TOP_OF_FREE_MEMORY, DEFAULT_HEADLESS_MEMTOP);
        self.ram
            .write_word(SAVMSC_SCREEN_MEMORY_POINTER, DEFAULT_HEADLESS_SCREEN);
    }

    fn cartridge_word(&self, address: u16) -> Option<u16> {
        let cartridge = self.cartridge.as_ref()?;
        let lo = cartridge.read(address)?;
        let hi = cartridge.read(address.wrapping_add(1))?;
        Some(u16::from_le_bytes([lo, hi]))
    }

    fn take_disk_boot_cartridge_redirect(&mut self, pc: u16) -> Option<u16> {
        if !self.redirect_disk_boot_to_cart {
            return None;
        }
        if !(SELF_TEST_BASE..=SELF_TEST_BASE + SELF_TEST_SIZE as u16 - 1).contains(&pc) {
            return None;
        }

        let target = self.cartridge_word(CARTCS_COLDSTART_VECTOR)?;
        self.redirect_disk_boot_to_cart = false;
        Some(target)
    }

    fn read_io(&mut self, address: u16) -> Option<u8> {
        if address == ANTIC_VCOUNT {
            let value = self.vcount;
            self.vcount = self.vcount.wrapping_add(1) & 0x7F;
            return Some(value);
        }

        self.io.read(address)
    }

    fn read_ram(&mut self, address: u16) -> u8 {
        if address == CH_KEY_CODE {
            if self.ram.read(CH_KEY_CODE) == 0xFF {
                if let Some(key_code) = self.pending_key_codes.pop_front() {
                    self.deliver_key_code(key_code);
                } else if self.has_queued_input_device()
                    && let Some(key_code) = self
                        .scripted_cio_input
                        .front()
                        .and_then(|byte| atari_character_to_key_code(*byte))
                {
                    self.deliver_key_code(key_code);
                }
            }
        }

        let value = self.ram.read(address);
        if address == RTCLOK_LOW {
            self.ram.write(address, value.wrapping_add(1));
        }
        value
    }

    fn deliver_key_code(&mut self, key_code: u8) {
        self.ram.write(CH_KEY_CODE, key_code);
        self.record_event(BusAccess::Write, CH_KEY_CODE, key_code, BusRegion::Ram);
        self.ram.write(KBCODE_PRIOR_KEY_CODE, key_code);
        self.record_event(
            BusAccess::Write,
            KBCODE_PRIOR_KEY_CODE,
            key_code,
            BusRegion::Ram,
        );
    }

    fn pop_scripted_cio_input_byte(&mut self) -> Option<u8> {
        self.scripted_cio_input.pop_front()
    }

    fn try_open_harness_cio_device(&mut self, x: u8) -> bool {
        let Some(channel) = cio_channel_index(x) else {
            return false;
        };
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(x as u16));
        let length = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(x as u16));
        let (spec_buffer, spec_length) = self.cio_spec_buffer(buffer, length);
        let raw0 = self.peek_mapped(buffer);
        let raw1 = self.peek_mapped(buffer.wrapping_add(1));
        let raw2 = self.peek_mapped(buffer.wrapping_add(2));
        self.trace_cio(format_args!(
            "  open spec raw=${:02X} ${:02X} ${:02X} start=${spec_buffer:04X} len={spec_length}",
            raw0, raw1, raw2
        ));
        let device = match self.peek_mapped(spec_buffer).to_ascii_uppercase() {
            b'Q' => CioHarnessDevice::QueuedInput,
            b'H' => {
                let spec = self.read_iocb_string(spec_buffer, spec_length);
                let name = normalize_host_file_name(&spec);
                let Some(file_index) = self.host_file_lookup.get(&name).copied() else {
                    self.trace_cio(format_args!("  H: open miss spec=`{spec}` name=`{name}`"));
                    return false;
                };
                self.trace_cio(format_args!("  H: open spec=`{spec}` name=`{name}`"));
                if self.host_files[file_index].writable && self.open_mode_is_write(x) {
                    self.host_files[file_index].bytes.clear();
                }
                CioHarnessDevice::Host {
                    file_index,
                    offset: 0,
                }
            }
            _ => return false,
        };
        if self.peek_mapped(spec_buffer.wrapping_add(1)) != b':' {
            return false;
        }

        self.cio_harness_devices[channel] = Some(device);
        self.trace_cio(format_args!("  harness open channel={channel} device={device:?}"));
        true
    }

    fn close_harness_cio_device(&mut self, x: u8) -> bool {
        let Some(channel) = cio_channel_index(x) else {
            return false;
        };
        let was_open = self.cio_harness_devices[channel].is_some();
        self.cio_harness_devices[channel] = None;
        was_open
    }

    fn cio_channel_device(&self, x: u8) -> Option<CioHarnessDevice> {
        cio_channel_index(x).and_then(|channel| self.cio_harness_devices[channel])
    }

    fn open_mode_is_write(&self, x: u8) -> bool {
        self.ram.read(IOCB_AUX1_BASE.wrapping_add(x as u16)) & 0x08 != 0
    }

    fn has_queued_input_device(&self) -> bool {
        self.cio_harness_devices
            .iter()
            .any(|device| matches!(device, Some(CioHarnessDevice::QueuedInput)))
    }

    fn read_iocb_string(&mut self, buffer: u16, length: u16) -> String {
        let max_len = if length == 0 { 64 } else { length.min(255) };
        let mut bytes = Vec::new();
        for offset in 0..max_len {
            let byte = self.peek_mapped(buffer.wrapping_add(offset));
            if byte == 0 || byte == 0x9B {
                break;
            }
            bytes.push(byte & 0x7F);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn cio_spec_buffer(&mut self, buffer: u16, length: u16) -> (u16, u16) {
        let first = self.peek_mapped(buffer);
        if length > 0
            && first == length as u8
            && self.peek_mapped(buffer.wrapping_add(2)) == b':'
        {
            (buffer.wrapping_add(1), length)
        } else {
            (buffer, length)
        }
    }

    fn peek_mapped(&mut self, address: u16) -> u8 {
        self.read(address)
    }

    fn read_host_character(&mut self, x: u8) -> Option<(u8, u8)> {
        let channel = cio_channel_index(x)?;
        let Some(CioHarnessDevice::Host { file_index, offset }) = self.cio_harness_devices[channel]
        else {
            return None;
        };
        let file = self.host_files.get(file_index)?;
        if file.writable {
            return None;
        }
        let mut next_offset = offset;
        while next_offset < file.bytes.len() {
            let byte = file.bytes[next_offset];
            next_offset += 1;
            if byte == b'\r' {
                continue;
            }
            self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host {
                file_index,
                offset: next_offset,
            });
            return Some((host_source_byte_to_atascii(byte), 0x01));
        }
        Some((0x88, 0x88))
    }

    fn read_host_record(&mut self, x: u8) -> Option<(u8, u8)> {
        let channel = cio_channel_index(x)?;
        let Some(CioHarnessDevice::Host { file_index, offset }) = self.cio_harness_devices[channel]
        else {
            return None;
        };
        let requested = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(x as u16));
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(x as u16));
        let file = self.host_files.get(file_index)?;
        if file.writable {
            return None;
        }
        if requested == 0 || offset >= file.bytes.len() {
            self.ram
                .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), 0);
            return Some((0x88, 0x88));
        }

        let mut next_offset = offset;
        let mut written = 0u16;
        let mut wrote_eol = false;
        while written < requested && next_offset < file.bytes.len() {
            let byte = file.bytes[next_offset];
            next_offset += 1;
            if byte == b'\r' {
                continue;
            }
            let output = host_source_byte_to_atascii(byte);
            self.ram.write(buffer.wrapping_add(written), output);
            written = written.wrapping_add(1);
            if output == 0x9B {
                wrote_eol = true;
                break;
            }
        }

        if written == 0 {
            self.ram
                .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), 0);
            return Some((0x88, 0x88));
        }

        if !wrote_eol && written < requested {
            self.ram.write(buffer.wrapping_add(written), 0x9B);
            written = written.wrapping_add(1);
        }

        self.ram
            .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), written);
        self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host {
            file_index,
            offset: next_offset,
        });
        Some((0, 0x01))
    }

    fn write_host_bytes_for_iocb(&mut self, x: u8, accumulator: u8) -> Option<()> {
        let channel = cio_channel_index(x)?;
        let Some(CioHarnessDevice::Host { file_index, offset }) = self.cio_harness_devices[channel]
        else {
            return None;
        };
        if !self.host_files.get(file_index)?.writable {
            return None;
        }

        let bytes = self.cio_output_bytes_for_iocb(x, accumulator);
        self.host_files[file_index].bytes.extend_from_slice(&bytes);
        self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host {
            file_index,
            offset: offset.saturating_add(bytes.len()),
        });
        self.trace_cio(format_args!(
            "  H: wrote {} byte(s) to `{}`",
            bytes.len(),
            self.host_files[file_index].name
        ));
        Some(())
    }

    fn cio_output_bytes_for_iocb(&self, x: u8, accumulator: u8) -> Vec<u8> {
        let base = x as u16;
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(base));
        let length = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(base));
        if buffer == 0 || length == 0 {
            return vec![accumulator];
        }

        let mut bytes = Vec::with_capacity(length as usize);
        for offset in 0..length {
            bytes.push(self.ram.read(buffer.wrapping_add(offset)));
        }
        bytes
    }

    fn capture_cio_channel0_output(&mut self, bytes: &[u8]) {
        self.ensure_text_cursor_defaults();
        self.cio_channel0_output.extend(bytes);
        for byte in bytes {
            if *byte == 0x9B {
                self.ram.write(COLCRS, 0);
                self.ram.write(ROWCRS, self.ram.read(ROWCRS).wrapping_add(1));
            } else {
                self.ram.write(COLCRS, self.ram.read(COLCRS).wrapping_add(1));
            }
        }
    }

    fn ensure_text_cursor_defaults(&mut self) {
        if self.ram.read(RMARGIN) == 0 {
            self.ram.write(RMARGIN, 39);
        }
    }

    fn trace_cio_call(&self, x: u8, command: u8, return_pc: u16) {
        if !self.trace_cio {
            return;
        }
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(x as u16));
        let length = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(x as u16));
        let aux1 = self.ram.read(IOCB_AUX1_BASE.wrapping_add(x as u16));
        let aux2 = self.ram.read(IOCB_AUX2_BASE.wrapping_add(x as u16));
        eprintln!(
            "CIO x=${x:02X} ch={} cmd=${command:02X} ret=${return_pc:04X} aux=${aux1:02X}/${aux2:02X} buf=${buffer:04X} len={length} dev={:?}",
            cio_channel_index(x).map_or(0xFF, |channel| channel as u8),
            self.cio_channel_device(x)
        );
    }

    fn trace_cio(&self, args: std::fmt::Arguments<'_>) {
        if self.trace_cio {
            eprintln!("{args}");
        }
    }

    fn read_self_test(&self, address: u16) -> Option<u8> {
        if !AddressRange::with_size(SELF_TEST_BASE, SELF_TEST_SIZE)
            .expect("valid self-test range")
            .contains(address)
        {
            return None;
        }
        if self.io.portb() & PORTB_SELF_TEST_DISABLE != 0 {
            return None;
        }

        let os_address = IO_BASE.wrapping_add(address - SELF_TEST_BASE);
        self.os_rom.as_ref()?.read(os_address)
    }

    fn display_list_screen_base(&self) -> Option<u16> {
        let display_list = ram_address(self.ram.read_word(SDLSTL_DISPLAY_LIST_POINTER))?;
        for offset in 0..256u16 {
            let instruction = self.ram.read(display_list.wrapping_add(offset));
            let mode = instruction & 0x0F;
            if instruction & 0x40 == 0 || mode < 2 {
                continue;
            }
            let lo = self
                .ram
                .read(display_list.wrapping_add(offset.wrapping_add(1)));
            let hi = self
                .ram
                .read(display_list.wrapping_add(offset.wrapping_add(2)));
            let screen = u16::from_le_bytes([lo, hi]);
            if let Some(screen) = ram_address(screen) {
                return Some(screen);
            }
        }
        None
    }

    fn decoded_ram_line_containing_action_error(&self) -> Option<String> {
        let pattern = [0x25, 0x72, 0x72, 0x6F, 0x72, 0x1A];
        for address in 0..=u16::MAX.wrapping_sub(pattern.len() as u16) {
            if !pattern.iter().enumerate().all(|(offset, expected)| {
                self.ram.read(address.wrapping_add(offset as u16)) & 0x7F == *expected
            }) {
                continue;
            }
            let mut line = String::with_capacity(40);
            for offset in 0..40u16 {
                line.push(atari_screen_code_to_ascii(
                    self.ram.read(address.wrapping_add(offset)),
                ));
            }
            return Some(format!("${address:04X}: {}", line.trim_end()));
        }
        None
    }

    fn action_line_max(&self) -> u8 {
        let line_max = self.ram.read(ACTION_LINEMAX);
        if line_max == 0 { 120 } else { line_max }
    }

    fn allocate_action_heap(&mut self, requested_size: u16) -> Result<u16, String> {
        if requested_size < 4 {
            return Err("Action! heap allocation request is too small".to_string());
        }

        let mut last = ACTION_AFBASE;
        let mut current = self.ram.read_word(last);
        while current != 0 {
            let next = self.ram.read_word(current);
            let size = self.ram.read_word(current.wrapping_add(2)) & 0x7FFF;
            if size >= requested_size {
                let remaining = size - requested_size;
                if remaining >= 4 {
                    let remainder = current.wrapping_add(requested_size);
                    self.ram.write_word(last, remainder);
                    self.ram.write_word(remainder, next);
                    self.ram.write_word(remainder.wrapping_add(2), remaining);
                    self.ram.write_word(current.wrapping_add(2), requested_size);
                } else {
                    self.ram.write_word(last, next);
                    self.ram.write_word(current.wrapping_add(2), size);
                }
                return Ok(current);
            }

            last = current;
            current = next;
        }

        Err(format!(
            "Action! heap has no free block large enough for {requested_size} byte(s)"
        ))
    }

    fn free_action_line_list(&mut self, top: u16) -> Result<(), String> {
        let mut address = top;
        for _ in 0..1024 {
            if address == 0 {
                return Ok(());
            }
            let next = self.ram.read_word(address.wrapping_add(4));
            self.free_action_heap(address)?;
            address = next;
        }
        Err("existing Action! editor line list did not terminate within 1024 lines".to_string())
    }

    fn free_action_heap(&mut self, address: u16) -> Result<(), String> {
        let mut last = ACTION_AFBASE;
        let mut current = self.ram.read_word(last);
        while current != 0 && current < address {
            last = current;
            current = self.ram.read_word(current);
        }

        self.ram.write_word(address, current);
        self.ram.write_word(last, address);
        self.coalesce_action_free_blocks(address);
        if last != ACTION_AFBASE {
            self.coalesce_action_free_blocks(last);
        }
        Ok(())
    }

    fn coalesce_action_free_blocks(&mut self, start: u16) {
        let mut block = start;
        for _ in 0..2 {
            let next = self.ram.read_word(block);
            if next == 0 {
                return;
            }
            let size = self.ram.read_word(block.wrapping_add(2)) & 0x7FFF;
            if block.wrapping_add(size) != next {
                block = next;
                continue;
            }
            let next_size = self.ram.read_word(next.wrapping_add(2)) & 0x7FFF;
            let after_next = self.ram.read_word(next);
            self.ram.write_word(block, after_next);
            self.ram
                .write_word(block.wrapping_add(2), size.wrapping_add(next_size));
        }
    }

    fn write_action_scratch_line(&mut self, line: &[u8]) -> Result<(), String> {
        let buf = self.ram.read_word(ACTION_BUF);
        if buf == 0 {
            return Ok(());
        }
        self.ram.write(buf, line.len() as u8);
        if line.is_empty() {
            Ok(())
        } else {
            self.ram.map(buf.wrapping_add(1), line)
        }
    }
}

fn split_action_source_lines(source: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    for raw_line in source.split(|byte| *byte == b'\n') {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        lines.push(line);
    }
    if source.ends_with(b"\n") {
        lines.pop();
    }
    lines
}

fn ram_address(address: u16) -> Option<u16> {
    if address != 0 && address < OS_ROM_BASE {
        Some(address)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostFile {
    name: String,
    bytes: Vec<u8>,
    writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CioHarnessDevice {
    QueuedInput,
    Host { file_index: usize, offset: usize },
}

fn normalize_host_file_name(name: &str) -> String {
    let trimmed = name.trim();
    let without_device = trimmed
        .strip_prefix("H:")
        .or_else(|| trimmed.strip_prefix("h:"))
        .unwrap_or(trimmed);
    without_device.trim().to_ascii_uppercase()
}

fn host_source_byte_to_atascii(byte: u8) -> u8 {
    match byte {
        b'\n' => 0x9B,
        _ => byte,
    }
}

fn atari_debug_char(byte: u8) -> char {
    match byte {
        0x9B => '\u{23CE}',
        0x20..=0x7E => byte as char,
        _ => '.',
    }
}

fn cio_channel_index(x: u8) -> Option<usize> {
    if x & 0x0F == 0 && x <= 0x70 {
        Some((x >> 4) as usize)
    } else {
        None
    }
}

fn atari_screen_code_to_ascii(value: u8) -> char {
    let code = value & 0x7F;
    match code {
        0x00..=0x3F => (code + 0x20) as char,
        0x60..=0x7A => code as char,
        _ => '.',
    }
}

fn atari_key_code_to_character(key_code: u8) -> Option<u8> {
    match key_code {
        ACTION_MONITOR_KEY_CODE => Some(ACTION_MONITOR_KEY_CODE),
        ATARI_KEY_C => Some(b'C'),
        ATARI_KEY_E => Some(b'E'),
        ATARI_KEY_RETURN => Some(0x9B),
        _ => None,
    }
}

fn atari_character_to_key_code(character: u8) -> Option<u8> {
    match character {
        b'C' | b'c' => Some(ATARI_KEY_C),
        b'E' | b'e' => Some(ATARI_KEY_E),
        0x9B => Some(ATARI_KEY_RETURN),
        _ => Some((character & 0x3F) | 0x40),
    }
}

fn decode_atascii_output(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        match *byte {
            0x9B => output.push('\n'),
            0x1B => {}
            0x20..=0x7E => output.push(*byte as char),
            _ => output.push('.'),
        }
    }
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusAccess {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusRegion {
    Ram,
    Io,
    SelfTestRom,
    OsRom,
    Cartridge,
    CartridgeControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusEvent {
    pub access: BusAccess,
    pub address: u16,
    pub value: u8,
    pub region: BusRegion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomRegion {
    range: AddressRange,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoRegion {
    range: AddressRange,
    bytes: Vec<u8>,
    console_switches: u8,
    speaker_write_count: u64,
    last_speaker_write: Option<u8>,
}

impl Default for IoRegion {
    fn default() -> Self {
        Self {
            range: AddressRange::with_size(IO_BASE, IO_SIZE).expect("valid I/O range"),
            bytes: vec![0xFF; IO_SIZE],
            console_switches: CONSOL_NO_KEYS,
            speaker_write_count: 0,
            last_speaker_write: None,
        }
    }
}

impl IoRegion {
    pub fn contains(&self, address: u16) -> bool {
        self.range.contains(address)
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        if !self.contains(address) {
            return None;
        }
        if address == CONSOL {
            return Some(self.console_switches);
        }
        Some(self.bytes[(address - self.range.start) as usize])
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        if !self.contains(address) {
            return false;
        }
        if address == CONSOL {
            self.speaker_write_count += 1;
            self.last_speaker_write = Some(value);
            return true;
        }
        self.bytes[(address - self.range.start) as usize] = value;
        true
    }

    pub fn portb(&self) -> u8 {
        self.read(PORTB).expect("PORTB is inside I/O range")
    }

    pub fn speaker_write_count(&self) -> u64 {
        self.speaker_write_count
    }

    pub fn last_speaker_write(&self) -> Option<u8> {
        self.last_speaker_write
    }
}

impl RomRegion {
    pub fn new(base: u16, bytes: Vec<u8>) -> Result<Self, String> {
        let range = AddressRange::with_size(base, bytes.len())?;
        Ok(Self { range, bytes })
    }

    pub fn range(&self) -> AddressRange {
        self.range
    }

    pub fn contains(&self, address: u16) -> bool {
        self.range.contains(address)
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        if !self.contains(address) {
            return None;
        }
        Some(self.bytes[(address - self.range.start) as usize])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cartridge {
    header: Option<CarHeader>,
    mapping: CartridgeMapping,
}

impl Cartridge {
    fn from_loaded_image(image: &LoadedImage) -> Result<Self, String> {
        Self::from_payload(image.base, image.car_header, image.bytes.clone())
    }

    fn from_payload(
        base: u16,
        header: Option<CarHeader>,
        payload: Vec<u8>,
    ) -> Result<Self, String> {
        if payload.is_empty() {
            return Err("cartridge payload is empty".to_string());
        }

        let mapping = if header.is_some_and(|header| header.cartridge_type == 0x0F) {
            CartridgeMapping::OssType15(OssType15Cartridge::new(base, payload)?)
        } else if payload.len() == 0x4000 {
            CartridgeMapping::Banked8k(BankedCartridge::new(
                base,
                payload,
                OSS_BANKED_8K_WINDOW_SIZE,
            )?)
        } else {
            CartridgeMapping::Linear(RomRegion::new(base, payload)?)
        };

        Ok(Self { header, mapping })
    }

    pub fn header(&self) -> Option<CarHeader> {
        self.header
    }

    pub fn mapping_info(&self) -> CartridgeMappingInfo {
        match &self.mapping {
            CartridgeMapping::Linear(region) => CartridgeMappingInfo {
                window_start: region.range.start,
                window_end: region.range.end,
                bank_size: region.bytes.len(),
                bank_count: 1,
                active_bank: 0,
            },
            CartridgeMapping::Banked8k(cart) => cart.mapping_info(),
            CartridgeMapping::OssType15(cart) => cart.mapping_info(),
        }
    }

    pub fn contains(&self, address: u16) -> bool {
        match &self.mapping {
            CartridgeMapping::Linear(region) => region.contains(address),
            CartridgeMapping::Banked8k(cart) => cart.contains(address),
            CartridgeMapping::OssType15(cart) => cart.contains(address),
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match &self.mapping {
            CartridgeMapping::Linear(region) => region.read(address),
            CartridgeMapping::Banked8k(cart) => cart.read(address),
            CartridgeMapping::OssType15(cart) => cart.read(address),
        }
    }

    pub fn payload(&self) -> &[u8] {
        match &self.mapping {
            CartridgeMapping::Linear(region) => &region.bytes,
            CartridgeMapping::Banked8k(cart) => &cart.payload,
            CartridgeMapping::OssType15(cart) => &cart.payload,
        }
    }

    pub fn apply_hotpatch(&mut self, hotpatch: Hotpatch) -> Result<HotpatchReport, String> {
        match hotpatch {
            Hotpatch::ActionQueuedInput => self.patch_action_keyboard_device_to_queue(),
            Hotpatch::ActionHeadlessGetkey => self.patch_action_headless_getkey(),
        }
    }

    fn patch_action_keyboard_device_to_queue(&mut self) -> Result<HotpatchReport, String> {
        const PATTERN: &[u8] = &[0x02, b'K', b':', 0xAD, 0xFC, 0x02, 0x49, 0xFF, 0x60];
        const DEVICE_OFFSET: usize = 1;

        let payload = match &mut self.mapping {
            CartridgeMapping::Linear(region) => &mut region.bytes,
            CartridgeMapping::Banked8k(cart) => &mut cart.payload,
            CartridgeMapping::OssType15(cart) => &mut cart.payload,
        };
        let matches = payload
            .windows(PATTERN.len())
            .enumerate()
            .filter_map(|(offset, window)| (window == PATTERN).then_some(offset))
            .collect::<Vec<_>>();

        let [payload_offset] = matches.as_slice() else {
            return Err(format!(
                "action-q-input hotpatch expected one Action! `K:` keyboard device pattern, found {}",
                matches.len()
            ));
        };
        let device_offset = payload_offset + DEVICE_OFFSET;
        let old_value = payload[device_offset];
        payload[device_offset] = b'Q';
        Ok(HotpatchReport {
            patch: Hotpatch::ActionQueuedInput,
            payload_offset: device_offset,
            old_value,
            new_value: b'Q',
        })
    }

    fn patch_action_headless_getkey(&mut self) -> Result<HotpatchReport, String> {
        const PATTERN: &[u8] = &[
            0x18, 0xA5, 0x14, 0x69, 0x0E, 0xAA, 0xAD, 0xFC, 0x02, 0x49, 0xFF, 0xD0,
        ];
        const REPLACEMENT: &[u8] = &[
            0xA2, 0x70, // LDX #$70
            0xA9, 0x07, // LDA #GETCHR
            0x85, 0x11, // STA BRKKEY
            0x20, 0x40, 0xB3, // JSR GTKBD
            0x8D, 0xA2, 0x04, // STA CURCH
            0x60, // RTS
        ];

        let payload = match &mut self.mapping {
            CartridgeMapping::Linear(region) => &mut region.bytes,
            CartridgeMapping::Banked8k(cart) => &mut cart.payload,
            CartridgeMapping::OssType15(cart) => &mut cart.payload,
        };
        let matches = payload
            .windows(PATTERN.len())
            .enumerate()
            .filter_map(|(offset, window)| (window == PATTERN).then_some(offset))
            .collect::<Vec<_>>();

        let [payload_offset] = matches.as_slice() else {
            return Err(format!(
                "action-headless-getkey hotpatch expected one Action! GETKEY pattern, found {}",
                matches.len()
            ));
        };
        let old_value = payload[*payload_offset];
        let replacement_len = REPLACEMENT.len();
        payload[*payload_offset..*payload_offset + replacement_len].copy_from_slice(REPLACEMENT);
        Ok(HotpatchReport {
            patch: Hotpatch::ActionHeadlessGetkey,
            payload_offset: *payload_offset,
            old_value,
            new_value: REPLACEMENT[0],
        })
    }

    pub fn control_access(&mut self, address: u16) -> bool {
        match &mut self.mapping {
            CartridgeMapping::Linear(_) => false,
            CartridgeMapping::Banked8k(_) => false,
            CartridgeMapping::OssType15(cart) => cart.control_access(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match &mut self.mapping {
            CartridgeMapping::Linear(_) => false,
            CartridgeMapping::Banked8k(cart) => cart.write_control(address, value),
            CartridgeMapping::OssType15(cart) => cart.write_control(address, value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CartridgeMapping {
    Linear(RomRegion),
    Banked8k(BankedCartridge),
    OssType15(OssType15Cartridge),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OssType15Cartridge {
    bank_window: AddressRange,
    fixed_window: AddressRange,
    active_bank: Option<usize>,
    payload: Vec<u8>,
}

impl OssType15Cartridge {
    fn new(bank_window_start: u16, payload: Vec<u8>) -> Result<Self, String> {
        if payload.len() != 0x4000 {
            return Err(format!(
                "OSS type 15 cartridge payload must be 16K, got {} byte(s)",
                payload.len()
            ));
        }

        Ok(Self {
            bank_window: AddressRange::with_size(bank_window_start, OSS_TYPE_15_BANK_SIZE)?,
            fixed_window: AddressRange::with_size(OSS_TYPE_15_FIXED_BASE, OSS_TYPE_15_BANK_SIZE)?,
            active_bank: Some(0),
            payload,
        })
    }

    fn bank_count(&self) -> usize {
        (self.payload.len() - OSS_TYPE_15_BANK_SIZE) / OSS_TYPE_15_BANK_SIZE
    }

    fn contains(&self, address: u16) -> bool {
        self.active_bank.is_some()
            && (self.bank_window.contains(address) || self.fixed_window.contains(address))
    }

    fn read(&self, address: u16) -> Option<u8> {
        let active_bank = self.active_bank?;

        if self.fixed_window.contains(address) {
            let offset = (address - self.fixed_window.start) as usize;
            return self.payload.get(offset).copied();
        }

        if self.bank_window.contains(address) {
            let window_offset = (address - self.bank_window.start) as usize;
            let bank_offset =
                OSS_TYPE_15_BANK_SIZE + active_bank * OSS_TYPE_15_BANK_SIZE + window_offset;
            return self.payload.get(bank_offset).copied();
        }

        None
    }

    fn control_access(&mut self, address: u16) -> bool {
        if !(0xD500..=0xD5FF).contains(&address) {
            return false;
        }

        self.active_bank = match address & 0x0009 {
            0x0000 => Some(0),
            0x0001 => Some(2),
            0x0008 => None,
            0x0009 => Some(1),
            _ => unreachable!("masked OSS type 15 control address has only four values"),
        };
        true
    }

    fn write_control(&mut self, address: u16, _value: u8) -> bool {
        self.control_access(address)
    }

    fn mapping_info(&self) -> CartridgeMappingInfo {
        CartridgeMappingInfo {
            window_start: self.bank_window.start,
            window_end: self.fixed_window.end,
            bank_size: OSS_TYPE_15_BANK_SIZE,
            bank_count: self.bank_count(),
            active_bank: self.active_bank.unwrap_or(self.bank_count()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BankedCartridge {
    window: AddressRange,
    bank_size: usize,
    active_bank: usize,
    payload: Vec<u8>,
}

impl BankedCartridge {
    fn new(window_start: u16, payload: Vec<u8>, bank_size: usize) -> Result<Self, String> {
        if bank_size == 0 || payload.len() % bank_size != 0 {
            return Err(format!(
                "banked cartridge payload size {} is not a multiple of bank size {bank_size}",
                payload.len()
            ));
        }
        let window = AddressRange::with_size(window_start, bank_size)?;
        Ok(Self {
            window,
            bank_size,
            active_bank: 0,
            payload,
        })
    }

    fn bank_count(&self) -> usize {
        self.payload.len() / self.bank_size
    }

    fn contains(&self, address: u16) -> bool {
        self.window.contains(address)
    }

    fn read(&self, address: u16) -> Option<u8> {
        if !self.contains(address) {
            return None;
        }
        let window_offset = (address - self.window.start) as usize;
        let bank_offset = self.active_bank * self.bank_size + window_offset;
        self.payload.get(bank_offset).copied()
    }

    fn write_control(&mut self, address: u16, value: u8) -> bool {
        if !(0xD500..=0xD5FF).contains(&address) {
            return false;
        }

        let bank = (value as usize) & (self.bank_count() - 1);
        self.active_bank = bank;
        true
    }

    fn mapping_info(&self) -> CartridgeMappingInfo {
        CartridgeMappingInfo {
            window_start: self.window.start,
            window_end: self.window.end,
            bank_size: self.bank_size,
            bank_count: self.bank_count(),
            active_bank: self.active_bank,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressRange {
    pub start: u16,
    pub end: u16,
}

impl AddressRange {
    pub fn with_size(start: u16, size: usize) -> Result<Self, String> {
        let end = mapped_end(start, size)?;
        Ok(Self { start, end })
    }

    pub fn contains(&self, address: u16) -> bool {
        self.start <= address && address <= self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    bytes: Box<[u8; RAM_SIZE]>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            bytes: Box::new([0; RAM_SIZE]),
        }
    }
}

impl Memory {
    pub fn read(&self, address: u16) -> u8 {
        self.bytes[address as usize]
    }

    pub fn read_word(&self, address: u16) -> u16 {
        let lo = self.read(address);
        let hi = self.read(address.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.bytes[address as usize] = value;
    }

    pub fn write_word(&mut self, address: u16, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.write(address, lo);
        self.write(address.wrapping_add(1), hi);
    }

    pub fn map(&mut self, base: u16, bytes: &[u8]) -> Result<(), String> {
        mapped_end(base, bytes.len())?;
        let start = base as usize;
        let end = start + bytes.len();

        self.bytes[start..end].copy_from_slice(bytes);
        Ok(())
    }
}

fn mapped_end(base: u16, size: usize) -> Result<u16, String> {
    if size == 0 {
        return Err("image is empty".to_string());
    }

    let start = base as usize;
    let end_exclusive = start
        .checked_add(size)
        .ok_or_else(|| "image mapping overflows address space".to_string())?;

    if end_exclusive > RAM_SIZE {
        return Err(format!(
            "image at ${base:04X} with {size} byte(s) exceeds 64K address space"
        ));
    }

    Ok((end_exclusive - 1) as u16)
}

fn checksum16(bytes: &[u8]) -> u16 {
    bytes
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(*byte as u16))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_image_bytes_at_requested_base() {
        let mut memory = Memory::default();
        memory.map(0xA000, &[0x11, 0x22, 0x33]).unwrap();

        assert_eq!(memory.read(0x9FFF), 0x00);
        assert_eq!(memory.read(0xA000), 0x11);
        assert_eq!(memory.read(0xA001), 0x22);
        assert_eq!(memory.read(0xA002), 0x33);
    }

    #[test]
    fn rejects_images_that_cross_address_space_end() {
        let mut memory = Memory::default();
        let err = memory.map(0xFFFF, &[0x11, 0x22]).unwrap_err();

        assert!(err.contains("exceeds 64K"));
    }

    #[test]
    fn memory_reads_and_writes_words_little_endian() {
        let mut memory = Memory::default();

        memory.write_word(0x2000, 0x1234);

        assert_eq!(memory.read(0x2000), 0x34);
        assert_eq!(memory.read(0x2001), 0x12);
        assert_eq!(memory.read_word(0x2000), 0x1234);
    }

    #[test]
    fn computes_image_metadata() {
        let metadata = ImageMetadata::from_bytes(0xA000, &[0x11, 0x22, 0x33]).unwrap();

        assert_eq!(metadata.size, 3);
        assert_eq!(metadata.base, 0xA000);
        assert_eq!(metadata.end, 0xA002);
        assert_eq!(metadata.checksum16, 0x66);
        assert_eq!(metadata.crc32, 0xFAC7_3763);
    }

    #[test]
    fn run_configuration_requires_cartridge_and_os_rom() {
        let config = VmConfig::default();
        assert!(
            config
                .validate_for_execution()
                .unwrap_err()
                .contains("--cart")
        );

        let config = VmConfig {
            cartridge: Some(PathBuf::from("action.rom")),
            ..VmConfig::default()
        };
        assert!(
            config
                .validate_for_execution()
                .unwrap_err()
                .contains("--os")
        );

        let config = VmConfig {
            cartridge: Some(PathBuf::from("action.rom")),
            os_rom: Some(PathBuf::from("atarios.rom")),
            ..VmConfig::default()
        };
        config.validate_for_execution().unwrap();
    }

    #[test]
    fn action_os_preset_uses_common_rom_mapping() {
        let mut config = VmConfig {
            cartridge_base: 0x8000,
            os_base: 0xD000,
            ..VmConfig::default()
        };
        config.apply_preset(ACTION_OS_PRESET);

        assert_eq!(config.cartridge_base, 0xA000);
        assert_eq!(config.os_base, 0xC000);
    }

    #[test]
    fn prepares_car_container_as_banked_payload() {
        let image = LoadedImage::prepare(
            ImageKind::Cartridge,
            PathBuf::from("action.car"),
            0xA000,
            car_bytes(
                0x0F,
                &[
                    &[0x11; 0x1000],
                    &[0x22; 0x1000],
                    &[0x33; 0x1000],
                    &[0x44; 0x1000],
                ],
            ),
        )
        .unwrap();

        assert_eq!(
            image.car_header,
            Some(CarHeader {
                cartridge_type: 0x0F,
                checksum: 0x1234_5678,
            })
        );
        assert_eq!(image.bytes.len(), 0x4000);
        assert_eq!(image.metadata.base, 0xA000);
        assert_eq!(image.metadata.end, 0xBFFF);
        assert_eq!(
            image.cartridge_mapping,
            Some(CartridgeMappingInfo {
                window_start: 0xA000,
                window_end: 0xBFFF,
                bank_size: 0x1000,
                bank_count: 3,
                active_bank: 0,
            })
        );
    }

    #[test]
    fn action_q_input_hotpatch_rewrites_keyboard_device_string() {
        let mut payload = vec![0xFF; 0x4000];
        payload[0x3840..0x3849]
            .copy_from_slice(&[0x02, b'K', b':', 0xAD, 0xFC, 0x02, 0x49, 0xFF, 0x60]);
        let mut cartridge = Cartridge::from_payload(0xA000, None, payload).unwrap();

        let report = cartridge
            .apply_hotpatch(Hotpatch::ActionQueuedInput)
            .unwrap();

        assert_eq!(
            report,
            HotpatchReport {
                patch: Hotpatch::ActionQueuedInput,
                payload_offset: 0x3841,
                old_value: b'K',
                new_value: b'Q',
            }
        );
        assert_eq!(cartridge.payload()[0x3841], b'Q');
        assert_eq!(cartridge.payload()[0x3842], b':');
    }

    #[test]
    fn action_headless_getkey_hotpatch_rewrites_blinking_wait_loop() {
        let mut payload = vec![0xFF; 0x4000];
        payload[0x12F0..0x12FC].copy_from_slice(&[
            0x18, 0xA5, 0x14, 0x69, 0x0E, 0xAA, 0xAD, 0xFC, 0x02, 0x49, 0xFF, 0xD0,
        ]);
        let mut cartridge = Cartridge::from_payload(0xA000, None, payload).unwrap();

        let report = cartridge
            .apply_hotpatch(Hotpatch::ActionHeadlessGetkey)
            .unwrap();

        assert_eq!(
            report,
            HotpatchReport {
                patch: Hotpatch::ActionHeadlessGetkey,
                payload_offset: 0x12F0,
                old_value: 0x18,
                new_value: 0xA2,
            }
        );
        assert_eq!(
            &cartridge.payload()[0x12F0..0x12FD],
            &[
                0xA2, 0x70, 0xA9, 0x07, 0x85, 0x11, 0x20, 0x40, 0xB3, 0x8D, 0xA2, 0x04, 0x60,
            ]
        );
    }

    #[test]
    fn bus_reads_os_rom_and_ignores_os_rom_writes() {
        let mut bus = Bus::default();
        bus.map_os_rom(0xC000, vec![0xAA, 0xBB]).unwrap();

        assert_eq!(bus.read(0xC000), 0xAA);
        bus.write(0xC000, 0x44);
        assert_eq!(bus.read(0xC000), 0xAA);
    }

    #[test]
    fn bus_io_region_overrides_os_rom_hole() {
        let mut bus = Bus::default();
        bus.map_os_rom(0xC000, vec![0xAA; 0x4000]).unwrap();

        assert_eq!(bus.read(0xCFFF), 0xAA);
        assert_eq!(bus.read(0xD000), 0xFF);
        bus.write(0xD301, 0x7F);
        assert_eq!(bus.read(0xD301), 0x7F);
        assert_eq!(bus.read(0xD800), 0xAA);
    }

    #[test]
    fn console_switch_reads_are_independent_from_speaker_writes() {
        let mut bus = Bus::default();

        assert_eq!(bus.read(CONSOL), CONSOL_NO_KEYS);
        bus.write(CONSOL, 0x00);
        assert_eq!(bus.read(CONSOL), CONSOL_NO_KEYS);
        bus.write(CONSOL, 0x7F);
        assert_eq!(bus.read(CONSOL), CONSOL_NO_KEYS);
        assert_eq!(bus.speaker_write_count(), 2);
        assert_eq!(bus.last_speaker_write(), Some(0x7F));
    }

    #[test]
    fn pokey_serial_output_times_out_disk_boot_to_cartridge_coldstart() {
        let image = LoadedImage::prepare(
            ImageKind::Cartridge,
            PathBuf::from("action.car"),
            0xA000,
            car_bytes(
                0x0F,
                &[
                    &[0x11; 0x0FFA],
                    &[0x34, 0x12],
                    &[0x11; 0x04],
                    &[0x22; 0x1000],
                    &[0x33; 0x1000],
                    &[0x44; 0x1000],
                ],
            ),
        )
        .unwrap();
        let mut bus = Bus::default();
        bus.install_cartridge(Cartridge::from_loaded_image(&image).unwrap());

        bus.write(XMTDON_TRANSMISSION_DONE_FLAG, 0x00);
        bus.write(RECVDN_RECEIVE_DONE_FLAG, 0xFF);
        bus.write(TIMFLG_TIMEOUT_FLAG, 0x01);
        bus.write(SEROUT_SERIAL_OUTPUT, 0x31);

        assert_eq!(bus.read(XMTDON_TRANSMISSION_DONE_FLAG), 0xFF);
        assert_eq!(bus.read(RECVDN_RECEIVE_DONE_FLAG), 0x00);
        assert_eq!(bus.read(TIMFLG_TIMEOUT_FLAG), 0x00);
        assert_eq!(bus.read(BOOTQ_SUCCESSFUL_BOOT_FLAG), 0x01);
        assert_eq!(bus.read(DOSVEC_START_VECTOR), 0x34);
        assert_eq!(bus.read(DOSVEC_START_VECTOR.wrapping_add(1)), 0x12);
        assert_eq!(bus.read(RAMTOP_MEMORY_TOP_PAGE), DEFAULT_HEADLESS_RAMTOP_PAGE);
        assert_eq!(
            bus.ram().read_word(MEMTOP_OS_TOP_OF_FREE_MEMORY),
            DEFAULT_HEADLESS_MEMTOP
        );
        assert_eq!(
            bus.ram().read_word(SAVMSC_SCREEN_MEMORY_POINTER),
            DEFAULT_HEADLESS_SCREEN
        );

        bus.write(DOSVEC_START_VECTOR, 0x23);
        bus.write(DOSVEC_START_VECTOR.wrapping_add(1), 0xF2);
        assert_eq!(bus.read(DOSVEC_START_VECTOR), 0x34);
        assert_eq!(bus.read(DOSVEC_START_VECTOR.wrapping_add(1)), 0x12);
    }

    #[test]
    fn vm_redirects_self_test_fallback_to_cartridge_coldstart() {
        let mut fixed = vec![0xEA; 0x1000];
        fixed[0x0FFA] = 0xE7;
        fixed[0x0FFB] = 0xB7;
        let image = LoadedImage::prepare(
            ImageKind::Cartridge,
            PathBuf::from("action.car"),
            0xA000,
            car_bytes(
                0x0F,
                &[&fixed, &[0x22; 0x1000], &[0x33; 0x1000], &[0x44; 0x1000]],
            ),
        )
        .unwrap();
        let mut vm = CompilerVm::default();
        vm.bus
            .install_cartridge(Cartridge::from_loaded_image(&image).unwrap());
        vm.bus.redirect_disk_boot_to_cart = true;
        vm.cpu.registers.pc = SELF_TEST_BASE;

        let step = vm.step_cpu().unwrap();

        assert_eq!(step.pc, 0xB7E7);
        assert_eq!(step.opcode, 0xEA);
        assert_eq!(vm.cpu.registers().pc, 0xB7E8);
    }

    #[test]
    fn portb_maps_self_test_rom_from_hidden_os_slice() {
        let mut bus = Bus::default();
        let mut os_rom = vec![0xAA; 0x4000];
        os_rom[0x1000] = 0x4C;
        os_rom[0x1001] = 0x09;
        os_rom[0x1002] = 0x50;
        bus.map_os_rom(0xC000, os_rom).unwrap();

        assert_eq!(bus.io().portb(), 0xFF);
        assert_eq!(bus.read(0x5000), 0x00);
        bus.write(PORTB, 0x7F);
        assert_eq!(bus.io().portb(), 0x7F);
        assert_eq!(bus.read(0x5000), 0x4C);
        assert_eq!(bus.read(0x5001), 0x09);
        assert_eq!(bus.read(0x5002), 0x50);
        assert_eq!(bus.read(0xD000), 0xFF);
    }

    #[test]
    fn bus_advances_antic_vcount_on_reads() {
        let mut bus = Bus::default();

        assert_eq!(bus.read(ANTIC_VCOUNT), 0x00);
        assert_eq!(bus.read(ANTIC_VCOUNT), 0x01);
        bus.vcount = 0x7F;
        assert_eq!(bus.read(ANTIC_VCOUNT), 0x7F);
        assert_eq!(bus.read(ANTIC_VCOUNT), 0x00);
    }

    #[test]
    fn bus_advances_rtclok_low_on_reads() {
        let mut bus = Bus::default();

        assert_eq!(bus.read(RTCLOK_LOW), 0x00);
        assert_eq!(bus.read(RTCLOK_LOW), 0x01);
        bus.write(RTCLOK_LOW, 0xFE);
        assert_eq!(bus.read(RTCLOK_LOW), 0xFE);
        assert_eq!(bus.read(RTCLOK_LOW), 0xFF);
        assert_eq!(bus.read(RTCLOK_LOW), 0x00);
    }

    #[test]
    fn bus_latches_queued_key_code_until_ch_is_cleared() {
        let mut bus = Bus::default();
        bus.write(CH_KEY_CODE, 0xFF);
        bus.queue_key_code(0x21);

        assert_eq!(bus.read(CH_KEY_CODE), 0x21);
        assert_eq!(bus.read(CH_KEY_CODE), 0x21);
        assert_eq!(bus.read(KBCODE_PRIOR_KEY_CODE), 0x21);
        bus.write(CH_KEY_CODE, 0xFF);
        assert_eq!(bus.read(CH_KEY_CODE), 0xFF);
    }

    #[test]
    fn bus_returns_queued_key_codes_in_order_after_ch_is_cleared() {
        let mut bus = Bus::default();
        bus.write(CH_KEY_CODE, 0xFF);
        bus.queue_key_code(0x21);
        bus.queue_key_code(ACTION_MONITOR_KEY_CODE);

        assert_eq!(bus.read(CH_KEY_CODE), 0x21);
        bus.write(CH_KEY_CODE, 0xFF);
        assert_eq!(bus.read(CH_KEY_CODE), ACTION_MONITOR_KEY_CODE);
        assert_eq!(bus.read(KBCODE_PRIOR_KEY_CODE), ACTION_MONITOR_KEY_CODE);
    }

    #[test]
    fn bus_records_synthetic_key_delivery_writes() {
        let mut bus = Bus::default();
        bus.add_watchpoint(CH_KEY_CODE);
        bus.add_watchpoint(KBCODE_PRIOR_KEY_CODE);
        bus.write(CH_KEY_CODE, 0xFF);
        bus.queue_key_code(ACTION_MONITOR_KEY_CODE);

        assert_eq!(bus.read(CH_KEY_CODE), ACTION_MONITOR_KEY_CODE);

        assert!(bus.events().iter().any(|event| {
            event.access == BusAccess::Write
                && event.address == CH_KEY_CODE
                && event.value == ACTION_MONITOR_KEY_CODE
        }));
        assert!(bus.events().iter().any(|event| {
            event.access == BusAccess::Write
                && event.address == KBCODE_PRIOR_KEY_CODE
                && event.value == ACTION_MONITOR_KEY_CODE
        }));
    }

    #[test]
    fn bus_injects_action_source_as_editor_line_list() {
        let mut bus = Bus::default();
        bus.ram_mut().write_word(ACTION_AFBASE, 0x2000);
        bus.ram_mut().write_word(0x2000, 0);
        bus.ram_mut().write_word(0x2002, 0x1000);
        bus.ram_mut().write_word(ACTION_BUF, 0x3000);
        bus.ram_mut().write(ACTION_LINEMAX, 120);

        let report = bus.inject_action_source(b"PROC Main()\nRETURN\n").unwrap();
        let lines = bus.action_editor_lines().unwrap();

        assert_eq!(report.line_count, 2);
        assert_eq!(report.first_line, Some(0x2000));
        assert_eq!(report.last_line, Some(0x2012));
        assert_eq!(bus.ram().read_word(ACTION_TOP), 0x2000);
        assert_eq!(bus.ram().read_word(ACTION_BOT), 0x2012);
        assert_eq!(bus.ram().read_word(ACTION_CUR), 0x2000);
        assert_eq!(
            bus.ram()
                .read_word(ACTION_VARS_W1.wrapping_add(ACTION_WINDOW_CUR_OFFSET)),
            0x2000
        );
        assert_eq!(bus.ram().read(ACTION_VARS_TOP1), 0x20);
        assert_eq!(bus.ram().read_word(ACTION_AFBASE), 0x201F);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].previous, 0);
        assert_eq!(lines[0].next, 0x2012);
        assert_eq!(lines[0].text, b"PROC Main()");
        assert_eq!(lines[1].previous, 0x2000);
        assert_eq!(lines[1].next, 0);
        assert_eq!(lines[1].text, b"RETURN");
        assert_eq!(bus.ram().read(0x3000), 11);
        assert_eq!(bus.ram().read(0x3001), b'P');
    }

    #[test]
    fn bus_rejects_source_lines_over_action_line_limit() {
        let mut bus = Bus::default();
        bus.ram_mut().write_word(ACTION_AFBASE, 0x2000);
        bus.ram_mut().write_word(0x2000, 0);
        bus.ram_mut().write_word(0x2002, 0x1000);
        bus.ram_mut().write_word(ACTION_BUF, 0x3000);
        bus.ram_mut().write(ACTION_LINEMAX, 3);

        let err = bus.inject_action_source(b"TOO LONG").unwrap_err();

        assert!(err.contains("exceeding Action! line limit 3"));
    }

    #[test]
    fn bus_replaces_existing_action_source_lines() {
        let mut bus = Bus::default();
        bus.ram_mut().write_word(ACTION_AFBASE, 0x2000);
        bus.ram_mut().write_word(0x2000, 0);
        bus.ram_mut().write_word(0x2002, 0x1000);
        bus.ram_mut().write_word(ACTION_BUF, 0x3000);
        bus.ram_mut().write(ACTION_LINEMAX, 120);

        bus.inject_action_source(b"FIRST\nSECOND\n").unwrap();
        let report = bus.inject_action_source(b"NEW\n").unwrap();
        let lines = bus.action_editor_lines().unwrap();

        assert_eq!(report.line_count, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].address, 0x2000);
        assert_eq!(lines[0].text, b"NEW");
        assert_eq!(bus.ram().read_word(ACTION_AFBASE), 0x200A);
    }

    #[test]
    fn bus_decodes_text_screen_and_detects_visible_action_error() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write_word(SAVMSC_SCREEN_MEMORY_POINTER, 0x4000);
        bus.ram_mut()
            .map(
                0x4000,
                &[0x25, 0x72, 0x72, 0x6F, 0x72, 0x1A, 0x00, 0x11, 0x17],
            )
            .unwrap();

        let snapshot = bus.text_screen_snapshot(9, 1);

        assert_eq!(snapshot.base, 0x4000);
        assert_eq!(snapshot.lines, vec!["Error: 17"]);
        assert_eq!(bus.visible_action_error(), Some("Error: 17".to_string()));
    }

    #[test]
    fn bus_prefers_display_list_lms_for_text_screen_base() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write_word(SAVMSC_SCREEN_MEMORY_POINTER, 0xFC40);
        bus.ram_mut()
            .write_word(SDLSTL_DISPLAY_LIST_POINTER, 0x3000);
        bus.ram_mut()
            .map(0x3000, &[0x70, 0x70, 0x42, 0x00, 0x40])
            .unwrap();
        bus.ram_mut().map(0x4000, &[0x28, 0x29]).unwrap();

        let snapshot = bus.text_screen_snapshot(2, 1);

        assert_eq!(snapshot.base, 0x4000);
        assert_eq!(snapshot.lines, vec!["HI"]);
    }

    #[test]
    fn bus_finds_visible_action_error_by_scanning_ram_when_screen_base_is_invalid() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write_word(SAVMSC_SCREEN_MEMORY_POINTER, 0xFC40);
        bus.ram_mut()
            .map(
                0x4800,
                &[0x25, 0x72, 0x72, 0x6F, 0x72, 0x1A, 0x00, 0x11, 0x17],
            )
            .unwrap();

        assert_eq!(
            bus.visible_action_error(),
            Some("$4800: Error: 17".to_string())
        );
    }

    #[test]
    fn bus_reads_banked_cartridge_window_without_os_overlap() {
        let image = LoadedImage::prepare(
            ImageKind::Cartridge,
            PathBuf::from("action.car"),
            0xA000,
            car_bytes(
                0x0F,
                &[
                    &[0x11; 0x1000],
                    &[0x22; 0x1000],
                    &[0x33; 0x1000],
                    &[0x44; 0x1000],
                ],
            ),
        )
        .unwrap();
        let mut bus = Bus::default();
        bus.map_os_rom(0xC000, vec![0xCC; 0x4000]).unwrap();
        bus.install_cartridge(Cartridge::from_loaded_image(&image).unwrap());

        assert_eq!(bus.read(0xA000), 0x22);
        assert_eq!(bus.read(0xAFFF), 0x22);
        assert_eq!(bus.read(0xBFFF), 0x11);
        assert_eq!(bus.read(0xC000), 0xCC);

        bus.write(0xD501, 0x00);
        assert_eq!(bus.read(0xA000), 0x44);
        assert_eq!(bus.read(0xBFFF), 0x11);
        assert_eq!(bus.read(0xC000), 0xCC);

        bus.read(0xD509);
        assert_eq!(bus.read(0xA000), 0x33);
        assert_eq!(bus.read(0xBFFF), 0x11);
        assert_eq!(bus.read(0xC000), 0xCC);

        bus.write(0xD508, 0x00);
        assert_eq!(bus.read(0xA000), 0x00);
        assert_eq!(bus.read(0xBFFF), 0x00);
    }

    #[test]
    fn bus_records_watchpoint_events() {
        let mut bus = Bus::default();
        bus.add_watchpoint(0x000E);

        bus.write(0x000E, 0x30);
        assert_eq!(bus.read(0x000E), 0x30);

        assert_eq!(
            bus.events(),
            &[
                BusEvent {
                    access: BusAccess::Write,
                    address: 0x000E,
                    value: 0x30,
                    region: BusRegion::Ram,
                },
                BusEvent {
                    access: BusAccess::Read,
                    address: 0x000E,
                    value: 0x30,
                    region: BusRegion::Ram,
                },
            ]
        );
    }

    #[test]
    fn cpu_resets_from_reset_vector() {
        let mut bus = Bus::default();
        bus.ram_mut().write(0xFFFC, 0x34);
        bus.ram_mut().write(0xFFFD, 0x12);
        let mut cpu = Cpu::default();

        cpu.reset(&mut bus);

        assert_eq!(cpu.registers().pc, 0x1234);
        assert_eq!(cpu.registers().sp, 0xFD);
        assert_eq!(cpu.cycles(), 7);
    }

    #[test]
    fn cpu_steps_through_basic_program_via_bus() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x42, // LDA #$42
                    0x85, 0x10, // STA $10
                    0xA2, 0x7F, // LDX #$7F
                    0x86, 0x11, // STX $11
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        let first = cpu.step(&mut bus).unwrap();
        assert_eq!(first.pc, 0x0200);
        assert_eq!(first.opcode, 0xA9);
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        assert_eq!(bus.ram().read(0x0010), 0x42);
        assert_eq!(bus.ram().read(0x0011), 0x7F);
        assert_eq!(cpu.registers().pc, 0x0208);
    }

    #[test]
    fn cpu_adc_absolute_x_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x7F, // LDA #$7F
                    0xA2, 0x02, // LDX #$02
                    0x18, // CLC
                    0x7D, 0x10, 0x03, // ADC $0310,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0312, 0x01);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert!(registers.status & StatusFlags::OVERFLOW.bits() != 0);
        assert_eq!(registers.status & StatusFlags::CARRY.bits(), 0);
    }

    #[test]
    fn cpu_adc_indirect_y_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x40, // LDA #$40
                    0xA0, 0x01, // LDY #$01
                    0x71, 0x20, // ADC ($20),Y
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0020, 0x00);
        bus.ram_mut().write(0x0021, 0x30);
        bus.ram_mut().write(0x3001, 0x40);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert!(registers.status & StatusFlags::OVERFLOW.bits() != 0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
    }

    #[test]
    fn cpu_adc_zero_page_x_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x7F, // LDA #$7F
                    0xA2, 0x02, // LDX #$02
                    0x18, // CLC
                    0x75, 0x40, // ADC $40,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0042, 0x01);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert!(registers.status & StatusFlags::OVERFLOW.bits() != 0);
        assert_eq!(registers.status & StatusFlags::CARRY.bits(), 0);
    }

    #[test]
    fn cpu_adc_absolute_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0xFE, // LDA #$FE
                    0x18, // CLC
                    0x6D, 0x10, 0x03, // ADC $0310
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0310, 0x03);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x01);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_ora_absolute_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x40, // LDA #$40
                    0x0D, 0x20, 0x03, // ORA $0320
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0320, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0xC0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_ldy_absolute_loads_y() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xAC, 0x20, 0x03, // LDY $0320
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0320, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.y, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_ora_absolute_x_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x40, // LDA #$40
                    0xA2, 0x02, // LDX #$02
                    0x1D, 0x20, 0x03, // ORA $0320,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0322, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0xC0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_ora_indirect_y_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x40, // LDA #$40
                    0xA0, 0x02, // LDY #$02
                    0x11, 0x40, // ORA ($40),Y
                ],
            )
            .unwrap();
        bus.ram_mut().write_word(0x0040, 0x0320);
        bus.ram_mut().write(0x0322, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0xC0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_rol_zero_page_rotates_through_carry() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0x38, // SEC
                    0x26, 0x40, // ROL $40
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0040), 0x01);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_shift_rotate_symmetric_forms() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0x38, // SEC
                    0xA9, 0x40, // LDA #$40
                    0x2A, // ROL A
                    0x2E, 0x20, 0x03, // ROL $0320
                    0xA2, 0x02, // LDX #$02
                    0x36, 0x40, // ROL $40,X
                    0x66, 0x40, // ROR $40
                    0x18, // CLC
                    0x6E, 0x21, 0x03, // ROR $0321
                    0x4E, 0x22, 0x03, // LSR $0322
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x01);
        bus.ram_mut().write(0x0042, 0x40);
        bus.ram_mut().write(0x0320, 0x80);
        bus.ram_mut().write(0x0321, 0x02);
        bus.ram_mut().write(0x0322, 0x01);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        for _ in 0..10 {
            cpu.step(&mut bus).unwrap();
        }

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x81);
        assert_eq!(bus.ram().read(0x0320), 0x00);
        assert_eq!(bus.ram().read(0x0042), 0x81);
        assert_eq!(bus.ram().read(0x0040), 0x00);
        assert_eq!(bus.ram().read(0x0321), 0x01);
        assert_eq!(bus.ram().read(0x0322), 0x00);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
    }

    #[test]
    fn cpu_ror_absolute_x_rotates_through_carry() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0x38, // SEC
                    0xA2, 0x02, // LDX #$02
                    0x7E, 0x20, 0x03, // ROR $0320,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0322, 0x02);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0322), 0x81);
        assert_eq!(registers.status & StatusFlags::CARRY.bits(), 0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
    }

    #[test]
    fn cpu_asl_zero_page_shifts_memory_left() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0x06, 0x40, // ASL $40
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x40);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0040), 0x80);
        assert_eq!(registers.status & StatusFlags::CARRY.bits(), 0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
    }

    #[test]
    fn cpu_inc_absolute_increments_memory() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xEE, 0x20, 0x03, // INC $0320
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0320, 0xFF);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0320), 0x00);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_inc_absolute_x_increments_indexed_memory() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA2, 0x04, // LDX #$04
                    0xFE, 0x20, 0x03, // INC $0320,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0324, 0x7F);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0324), 0x80);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
    }

    #[test]
    fn cpu_dec_absolute_decrements_memory() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xCE, 0x20, 0x03, // DEC $0320
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0320, 0x00);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0320), 0xFF);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_dec_absolute_x_decrements_indexed_memory() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA2, 0x02, // LDX #$02
                    0xDE, 0x20, 0x03, // DEC $0320,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0322, 0x01);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(bus.ram().read(0x0322), 0x00);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_lsr_accumulator_shifts_right() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x01, // LDA #$01
                    0x4A, // LSR A
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x00);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_and_zero_page_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0xF0, // LDA #$F0
                    0x25, 0x40, // AND $40
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_and_absolute_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x0F, // LDA #$0F
                    0x2D, 0x20, 0x03, // AND $0320
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0320, 0xF0);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x00);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_and_indirect_y_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0xF3, // LDA #$F3
                    0xA0, 0x02, // LDY #$02
                    0x31, 0x40, // AND ($40),Y
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x20);
        bus.ram_mut().write(0x0041, 0x03);
        bus.ram_mut().write(0x0322, 0x0F);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x03);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_eor_indirect_y_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0xF0, // LDA #$F0
                    0xA0, 0x01, // LDY #$01
                    0x51, 0x40, // EOR ($40),Y
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x20);
        bus.ram_mut().write(0x0041, 0x03);
        bus.ram_mut().write(0x0321, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x70);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_eor_zero_page_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0xF0, // LDA #$F0
                    0x45, 0x40, // EOR $40
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x70);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_eor_absolute_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x0F, // LDA #$0F
                    0x4D, 0x00, 0x30, // EOR $3000
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x3000, 0xF0);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0xFF);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.pc, 0x0205);
    }

    #[test]
    fn cpu_and_absolute_x_updates_accumulator_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0xF0, // LDA #$F0
                    0xA2, 0x02, // LDX #$02
                    0x3D, 0x20, 0x03, // AND $0320,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0322, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_sbc_immediate_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x80, // LDA #$80
                    0x38, // SEC
                    0xE9, 0x01, // SBC #$01
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x7F);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
        assert!(registers.status & StatusFlags::OVERFLOW.bits() != 0);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
    }

    #[test]
    fn cpu_sbc_zero_page_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x10, // LDA #$10
                    0x38, // SEC
                    0xE5, 0x40, // SBC $40
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x01);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x0F);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_sbc_absolute_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x10, // LDA #$10
                    0x38, // SEC
                    0xED, 0x20, 0x03, // SBC $0320
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0320, 0x20);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0xF0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::CARRY.bits(), 0);
        assert_eq!(registers.status & StatusFlags::OVERFLOW.bits(), 0);
    }

    #[test]
    fn cpu_sbc_indirect_y_updates_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x10, // LDA #$10
                    0x38, // SEC
                    0xA0, 0x02, // LDY #$02
                    0xF1, 0x40, // SBC ($40),Y
                ],
            )
            .unwrap();
        bus.ram_mut().write_word(0x0040, 0x0320);
        bus.ram_mut().write(0x0322, 0x03);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x0D);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_bmi_branches_on_negative_flag() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x80, // LDA #$80
                    0x30, 0x02, // BMI +2
                    0xA9, 0x00, // skipped
                    0xA9, 0x11, // LDA #$11
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers().a, 0x11);
        assert_eq!(cpu.registers().pc, 0x0208);
    }

    #[test]
    fn cpu_cmp_absolute_y_sets_compare_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x20, // LDA #$20
                    0xA0, 0x03, // LDY #$03
                    0xD9, 0x10, 0x03, // CMP $0310,Y
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0313, 0x20);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x20);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_cmp_absolute_x_sets_compare_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x20, // LDA #$20
                    0xA2, 0x03, // LDX #$03
                    0xDD, 0x10, 0x03, // CMP $0310,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0313, 0x21);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.status & StatusFlags::CARRY.bits(), 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
    }

    #[test]
    fn cpu_cpy_zero_page_and_absolute_set_compare_flags() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA0, 0x40, // LDY #$40
                    0xC4, 0x20, // CPY $20
                    0xCC, 0x00, 0x30, // CPY $3000
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0020, 0x41);
        bus.ram_mut().write(0x3000, 0x40);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        let after_zero_page = cpu.registers();
        cpu.step(&mut bus).unwrap();
        let after_absolute = cpu.registers();

        assert_eq!(after_zero_page.status & StatusFlags::CARRY.bits(), 0);
        assert!(after_zero_page.status & StatusFlags::NEGATIVE.bits() != 0);
        assert!(after_absolute.status & StatusFlags::CARRY.bits() != 0);
        assert!(after_absolute.status & StatusFlags::ZERO.bits() != 0);
    }

    #[test]
    fn cpu_ldx_absolute_y_loads_indexed_value() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA0, 0x04, // LDY #$04
                    0xBE, 0x10, 0x03, // LDX $0310,Y
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0314, 0x80);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.x, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_lda_zero_page_x_loads_wrapped_value() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA2, 0x02, // LDX #$02
                    0xB5, 0xFF, // LDA $FF,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0001, 0x44);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x44);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
        assert_eq!(registers.status & StatusFlags::NEGATIVE.bits(), 0);
    }

    #[test]
    fn cpu_sta_zero_page_x_stores_wrapped_value() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x42, // LDA #$42
                    0xA2, 0x02, // LDX #$02
                    0x95, 0xFE, // STA $FE,X
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        assert_eq!(bus.ram().read(0x0000), 0x42);
    }

    #[test]
    fn cpu_tya_transfers_y_to_accumulator() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA0, 0x80, // LDY #$80
                    0x98, // TYA
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_tsx_transfers_stack_pointer_to_x() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA2, 0x80, // LDX #$80
                    0x9A, // TXS
                    0xA2, 0x00, // LDX #$00
                    0xBA, // TSX
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        for _ in 0..4 {
            cpu.step(&mut bus).unwrap();
        }

        let registers = cpu.registers();
        assert_eq!(registers.x, 0x80);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert_eq!(registers.status & StatusFlags::ZERO.bits(), 0);
    }

    #[test]
    fn cpu_pha_pushes_accumulator_to_stack() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x44, // LDA #$44
                    0x48, // PHA
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();

        assert_eq!(bus.ram().read(0x01FD), 0x44);
        assert_eq!(cpu.registers().sp, 0xFC);
    }

    #[test]
    fn cpu_stack_pop_and_status_round_trip() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x80, // LDA #$80
                    0x48, // PHA
                    0xA9, 0x00, // LDA #$00
                    0x68, // PLA
                    0x38, // SEC
                    0x08, // PHP
                    0x18, // CLC
                    0x28, // PLP
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        for _ in 0..8 {
            cpu.step(&mut bus).unwrap();
        }

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x80);
        assert_eq!(registers.sp, 0xFD);
        assert!(registers.status & StatusFlags::NEGATIVE.bits() != 0);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
    }

    #[test]
    fn cpu_emulates_keyboard_get_character_ciov_for_monitor_commands() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x70), CIO_COMMAND_GETCHR);
        bus.ram_mut().write(CH_KEY_CODE, ATARI_KEY_C);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x70;
        cpu.registers.sp = 0xFB;

        let step = cpu.step(&mut bus).unwrap();

        assert_eq!(step.pc, CIOV);
        assert_eq!(step.opcode, 0xFF);
        assert_eq!(cpu.registers().pc, 0x2000);
        assert_eq!(cpu.registers().a, b'C');
        assert_eq!(cpu.registers().y, 0x01);
        assert_eq!(bus.ram().read(CH_KEY_CODE), 0xFF);
    }

    #[test]
    fn cpu_emulates_scripted_cio_input_before_keyboard_latch() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x70), CIO_COMMAND_GETCHR);
        bus.ram_mut().write(CH_KEY_CODE, ATARI_KEY_C);
        bus.cio_harness_devices[7] = Some(CioHarnessDevice::QueuedInput);
        bus.queue_scripted_cio_input_byte(b'Q');
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x70;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers().pc, 0x2000);
        assert_eq!(cpu.registers().a, b'Q');
        assert_eq!(bus.ram().read(CH_KEY_CODE), ATARI_KEY_C);
    }

    #[test]
    fn bus_signals_key_down_when_queued_cio_input_is_pending() {
        let mut bus = Bus::default();
        bus.ram_mut().write(CH_KEY_CODE, 0xFF);
        bus.cio_harness_devices[7] = Some(CioHarnessDevice::QueuedInput);
        bus.queue_scripted_cio_input_byte(b'C');

        assert_eq!(bus.read(CH_KEY_CODE), ATARI_KEY_C);
        assert_eq!(bus.scripted_cio_input.front(), Some(&b'C'));
    }

    #[test]
    fn cpu_opens_and_closes_harness_cio_devices() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x20), CIO_COMMAND_OPEN);
        bus.ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(0x20), 0x3000);
        bus.ram_mut().map(0x3000, b"Q:").unwrap();
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x20;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(
            bus.cio_channel_device(0x20),
            Some(CioHarnessDevice::QueuedInput)
        );

        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x20), CIO_COMMAND_CLOSE);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = CIOV;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(bus.cio_channel_device(0x20), None);
    }

    #[test]
    fn cpu_writes_harness_host_output() {
        let mut bus = Bus::default();
        bus.add_host_output("OUT.COM");
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_OPEN);
        bus.ram_mut()
            .write(IOCB_AUX1_BASE.wrapping_add(0x10), 0x08);
        bus.ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(0x10), 0x3000);
        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x10), 9);
        bus.ram_mut().map(0x3000, b"H:OUT.COM").unwrap();
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x10;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_PUTCHR);
        bus.ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(0x10), 0x3100);
        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x10), 4);
        bus.ram_mut().map(0x3100, &[0xFF, 0xFF, 0x00, 0x30]).unwrap();
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x10;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(bus.host_file_bytes("OUT.COM"), Some(&[0xFF, 0xFF, 0x00, 0x30][..]));
    }

    #[test]
    fn cpu_captures_channel_zero_cio_output() {
        let mut bus = Bus::default();
        bus.ram_mut().write(IOCB_COMMAND_BASE, CIO_COMMAND_PUTREC);
        bus.ram_mut().write_word(IOCB_BUFFER_BASE, 0x3000);
        bus.ram_mut().write_word(IOCB_LENGTH_BASE, 3);
        bus.ram_mut().map(0x3000, b"OK\x9B").unwrap();
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x00;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers().pc, 0x2000);
        assert_eq!(bus.cio_channel0_output(), b"OK\x9B");
        assert_eq!(bus.decoded_cio_channel0_output(), "OK\n");
    }

    #[test]
    fn cpu_decimal_and_overflow_flags_have_clear_set_pairs() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xF8, // SED
                    0xD8, // CLD
                    0xA9, 0x7F, // LDA #$7F
                    0x18, // CLC
                    0x69, 0x01, // ADC #$01
                    0xB8, // CLV
                ],
            )
            .unwrap();
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        assert!(cpu.registers().status & StatusFlags::DECIMAL.bits() != 0);
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.registers().status & StatusFlags::DECIMAL.bits(), 0);
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        cpu.step(&mut bus).unwrap();
        assert!(cpu.registers().status & StatusFlags::OVERFLOW.bits() != 0);
        cpu.step(&mut bus).unwrap();
        assert_eq!(cpu.registers().status & StatusFlags::OVERFLOW.bits(), 0);
    }

    #[test]
    fn cpu_reports_unsupported_opcode_with_pc() {
        let mut bus = Bus::default();
        bus.ram_mut().write(0x0200, 0x02);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        assert_eq!(
            cpu.step(&mut bus).unwrap_err(),
            CpuError::UnsupportedOpcode {
                pc: 0x0200,
                opcode: 0x02,
            }
        );
        assert!(cpu.halted());
    }

    #[test]
    fn decodes_action_symbol_tables_from_official_table_shape() {
        let mut memory = Memory::default();
        memory.write_word(ACTION_GLOBAL_SYMBOL_TABLE_POINTER, 0x2000);
        memory.write(0x2001, 0x30);
        memory.write(0x2101, 0x00);
        write_symbol_entry(
            &mut memory,
            0x3000,
            "Plot",
            0xC0,
            Some(0xA6C3),
            &[4, 2],
        );

        memory.write_word(ACTION_LOCAL_SYMBOL_TABLE_POINTER, 0x2200);
        memory.write(0x2202, 0x31);
        memory.write(0x2302, 0x00);
        write_symbol_entry(&mut memory, 0x3100, "i", 0x82, Some(0x3028), &[]);

        let dump = decode_action_symbol_tables_from_memory(&memory);

        assert_eq!(dump.global_index, Some(0x2000));
        assert_eq!(dump.local_index, Some(0x2200));
        assert_eq!(dump.globals.len(), 1);
        assert_eq!(dump.locals.len(), 1);
        assert_eq!(dump.globals[0].scope, ActionSymbolScope::Global);
        assert_eq!(dump.globals[0].slot, 1);
        assert_eq!(dump.globals[0].name, "Plot");
        assert_eq!(dump.globals[0].address, Some(0xA6C3));
        assert_eq!(dump.globals[0].class, "PROC");
        assert_eq!(dump.globals[0].numargs, 2);
        assert_eq!(dump.globals[0].arg_types_raw, vec![4, 2]);
        assert_eq!(dump.globals[0].args, vec!["CARD", "BYTE"]);
        assert_eq!(dump.locals[0].scope, ActionSymbolScope::Local);
        assert_eq!(dump.locals[0].name, "i");
        assert_eq!(dump.locals[0].class, "BYTE");
    }

    #[test]
    fn formats_action_symbol_dump_as_json() {
        let mut memory = Memory::default();
        memory.write_word(ACTION_GLOBAL_SYMBOL_TABLE_POINTER, 0x2000);
        memory.write(0x2001, 0x30);
        memory.write(0x2101, 0x00);
        write_symbol_entry(&mut memory, 0x3000, "Main", 0xC0, Some(0x316C), &[]);

        let json = format_action_symbol_dump_json(&decode_action_symbol_tables_from_memory(&memory));

        assert!(json.contains("\"global_index\": \"$2000\""));
        assert!(json.contains("\"name\":\"Main\""));
        assert!(json.contains("\"address\":\"$316C\""));
        assert!(json.contains("\"locals\": []"));
    }

    fn car_bytes(cartridge_type: u32, chunks: &[&[u8]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CAR_MAGIC);
        bytes.extend_from_slice(&cartridge_type.to_be_bytes());
        bytes.extend_from_slice(&0x1234_5678u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 4]);
        for chunk in chunks {
            bytes.extend_from_slice(chunk);
        }
        bytes
    }

    fn write_symbol_entry(
        memory: &mut Memory,
        name_addr: u16,
        name: &str,
        vtype: u8,
        address: Option<u16>,
        args: &[u8],
    ) {
        memory.write(name_addr, name.len() as u8);
        for (offset, byte) in name.bytes().enumerate() {
            memory.write(name_addr.wrapping_add(1 + offset as u16), byte);
        }
        let entry = name_addr.wrapping_add(1 + name.len() as u16);
        memory.write(entry, vtype);
        if let Some(address) = address {
            memory.write_word(entry.wrapping_add(1), address);
        }
        if !args.is_empty() {
            memory.write(entry.wrapping_add(3), args.len() as u8);
            for (index, arg) in args.iter().copied().enumerate() {
                memory.write(entry.wrapping_add(4 + index as u16), arg);
            }
        }
    }
}
