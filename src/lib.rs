use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

pub const RAM_SIZE: usize = 0x10000;
pub const DEFAULT_CART_BASE: u16 = 0xA000;
pub const OS_ROM_BASE: u16 = 0xC000;
pub const IO_BASE: u16 = 0xD000;
pub const IO_SIZE: usize = 0x0800;
pub const SELF_TEST_BASE: u16 = 0x5000;
pub const SELF_TEST_SIZE: usize = 0x0800;
pub const PORTB: u16 = 0xD301;
pub const PORTB_SELF_TEST_DISABLE: u8 = 0x80;
pub const ANTIC_VCOUNT: u16 = 0xD40B;
pub const RTCLOK_LOW: u16 = 0x0014;
pub const KBCODE_PRIOR_KEY_CODE: u16 = 0x02F2;
pub const CH_KEY_CODE: u16 = 0x02FC;
pub const ACTION_MONITOR_KEY_CODE: u8 = 0xE5;
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

        if let Some(path) = &self.source {
            vm.source = Some(
                fs::read(path)
                    .map_err(|err| format!("failed to read source `{}`: {err}", path.display()))?,
            );
        }

        Ok(vm)
    }
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
        self.cpu.step(&mut self.bus)
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
        let opcode = self.fetch_byte(bus);

        match opcode {
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
            0x18 => {
                self.set_flag(StatusFlags::CARRY, false);
                self.cycles += 2;
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
            0x4E => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = value >> 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
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
            0xCD => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 4;
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
            0xE0 => {
                let value = self.fetch_byte(bus);
                self.compare(self.registers.x, value);
                self.cycles += 2;
            }
            0xE4 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.compare(self.registers.x, value);
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
            0xF8 => {
                self.set_flag(StatusFlags::DECIMAL, true);
                self.cycles += 2;
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
        self.pending_key_codes.push_back(key_code);
    }

    pub fn map_os_rom(&mut self, base: u16, bytes: Vec<u8>) -> Result<(), String> {
        self.os_rom = Some(RomRegion::new(base, bytes)?);
        Ok(())
    }

    pub fn install_cartridge(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    pub fn read(&mut self, address: u16) -> u8 {
        let (value, region) = if let Some(cartridge) = self.cartridge.as_ref() {
            if let Some(value) = cartridge.read(address) {
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
                    self.ram.write(CH_KEY_CODE, key_code);
                    self.ram.write(KBCODE_PRIOR_KEY_CODE, key_code);
                }
            }
        }

        let value = self.ram.read(address);
        if address == RTCLOK_LOW {
            self.ram.write(address, value.wrapping_add(1));
        }
        value
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
}

impl Default for IoRegion {
    fn default() -> Self {
        Self {
            range: AddressRange::with_size(IO_BASE, IO_SIZE).expect("valid I/O range"),
            bytes: vec![0xFF; IO_SIZE],
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
        Some(self.bytes[(address - self.range.start) as usize])
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        if !self.contains(address) {
            return false;
        }
        self.bytes[(address - self.range.start) as usize] = value;
        true
    }

    pub fn portb(&self) -> u8 {
        self.read(PORTB).expect("PORTB is inside I/O range")
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
    active_bank: usize,
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
            active_bank: 0,
            payload,
        })
    }

    fn bank_count(&self) -> usize {
        (self.payload.len() - OSS_TYPE_15_BANK_SIZE) / OSS_TYPE_15_BANK_SIZE
    }

    fn contains(&self, address: u16) -> bool {
        self.bank_window.contains(address) || self.fixed_window.contains(address)
    }

    fn read(&self, address: u16) -> Option<u8> {
        if self.fixed_window.contains(address) {
            let offset = (address - self.fixed_window.start) as usize;
            return self.payload.get(offset).copied();
        }

        if self.bank_window.contains(address) {
            let window_offset = (address - self.bank_window.start) as usize;
            let bank_offset =
                OSS_TYPE_15_BANK_SIZE + self.active_bank * OSS_TYPE_15_BANK_SIZE + window_offset;
            return self.payload.get(bank_offset).copied();
        }

        None
    }

    fn write_control(&mut self, address: u16, value: u8) -> bool {
        if !(0xD500..=0xD5FF).contains(&address) {
            return false;
        }

        self.active_bank = (value as usize) % self.bank_count();
        true
    }

    fn mapping_info(&self) -> CartridgeMappingInfo {
        CartridgeMappingInfo {
            window_start: self.bank_window.start,
            window_end: self.fixed_window.end,
            bank_size: OSS_TYPE_15_BANK_SIZE,
            bank_count: self.bank_count(),
            active_bank: self.active_bank,
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

    pub fn write(&mut self, address: u16, value: u8) {
        self.bytes[address as usize] = value;
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

        bus.write(0xD500, 0x01);
        assert_eq!(bus.read(0xA000), 0x33);
        assert_eq!(bus.read(0xBFFF), 0x11);
        assert_eq!(bus.read(0xC000), 0xCC);
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
                    0x66, 0x40, // ROR $40
                    0x18, // CLC
                    0x6E, 0x21, 0x03, // ROR $0321
                    0x4E, 0x22, 0x03, // LSR $0322
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x0040, 0x01);
        bus.ram_mut().write(0x0320, 0x80);
        bus.ram_mut().write(0x0321, 0x02);
        bus.ram_mut().write(0x0322, 0x01);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        for _ in 0..8 {
            cpu.step(&mut bus).unwrap();
        }

        let registers = cpu.registers();
        assert_eq!(registers.a, 0x81);
        assert_eq!(bus.ram().read(0x0320), 0x00);
        assert_eq!(bus.ram().read(0x0040), 0x80);
        assert_eq!(bus.ram().read(0x0321), 0x01);
        assert_eq!(bus.ram().read(0x0322), 0x00);
        assert!(registers.status & StatusFlags::CARRY.bits() != 0);
        assert!(registers.status & StatusFlags::ZERO.bits() != 0);
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
}
