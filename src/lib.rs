use std::fs;
use std::path::PathBuf;

pub const RAM_SIZE: usize = 0x10000;
pub const DEFAULT_CART_BASE: u16 = 0xA000;
pub const OS_ROM_BASE: u16 = 0xC000;
pub const OSS_BANKED_8K_WINDOW_SIZE: usize = 0x2000;
pub const CAR_HEADER_SIZE: usize = 16;
pub const CAR_MAGIC: &[u8; 4] = b"CART";
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
}

impl Default for CompilerVm {
    fn default() -> Self {
        Self {
            bus: Bus::default(),
            images: Vec::new(),
            source: None,
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
    os_rom: Option<RomRegion>,
    cartridge: Option<Cartridge>,
    watchpoints: Vec<u16>,
    events: Vec<BusEvent>,
    last_data: u8,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            ram: Memory::default(),
            os_rom: None,
            cartridge: None,
            watchpoints: Vec::new(),
            events: Vec::new(),
            last_data: 0,
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

    pub fn add_watchpoint(&mut self, address: u16) {
        if !self.watchpoints.contains(&address) {
            self.watchpoints.push(address);
        }
    }

    pub fn events(&self) -> &[BusEvent] {
        &self.events
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
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
            } else if let Some(os_rom) = self.os_rom.as_ref() {
                if let Some(value) = os_rom.read(address) {
                    (value, BusRegion::OsRom)
                } else {
                    (self.ram.read(address), BusRegion::Ram)
                }
            } else {
                (self.ram.read(address), BusRegion::Ram)
            }
        } else if let Some(os_rom) = self.os_rom.as_ref() {
            if let Some(value) = os_rom.read(address) {
                (value, BusRegion::OsRom)
            } else {
                (self.ram.read(address), BusRegion::Ram)
            }
        } else {
            (self.ram.read(address), BusRegion::Ram)
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
        if self.watchpoints.contains(&address) {
            self.events.push(BusEvent {
                access,
                address,
                value,
                region,
            });
        }
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

        let mapping = if header.is_some_and(|header| header.cartridge_type == 0x0F)
            || payload.len() == 0x4000
        {
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
        }
    }

    pub fn contains(&self, address: u16) -> bool {
        match &self.mapping {
            CartridgeMapping::Linear(region) => region.contains(address),
            CartridgeMapping::Banked8k(cart) => cart.contains(address),
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match &self.mapping {
            CartridgeMapping::Linear(region) => region.read(address),
            CartridgeMapping::Banked8k(cart) => cart.read(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match &mut self.mapping {
            CartridgeMapping::Linear(_) => false,
            CartridgeMapping::Banked8k(cart) => cart.write_control(address, value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CartridgeMapping {
    Linear(RomRegion),
    Banked8k(BankedCartridge),
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
            car_bytes(0x0F, &[0x11; 0x2000], &[0x22; 0x2000]),
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
                bank_size: 0x2000,
                bank_count: 2,
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
    fn bus_reads_banked_cartridge_window_without_os_overlap() {
        let image = LoadedImage::prepare(
            ImageKind::Cartridge,
            PathBuf::from("action.car"),
            0xA000,
            car_bytes(0x0F, &[0x11; 0x2000], &[0x22; 0x2000]),
        )
        .unwrap();
        let mut bus = Bus::default();
        bus.map_os_rom(0xC000, vec![0xCC; 0x4000]).unwrap();
        bus.install_cartridge(Cartridge::from_loaded_image(&image).unwrap());

        assert_eq!(bus.read(0xA000), 0x11);
        assert_eq!(bus.read(0xBFFF), 0x11);
        assert_eq!(bus.read(0xC000), 0xCC);

        bus.write(0xD500, 0x01);
        assert_eq!(bus.read(0xA000), 0x22);
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

    fn car_bytes(cartridge_type: u32, bank0: &[u8], bank1: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CAR_MAGIC);
        bytes.extend_from_slice(&cartridge_type.to_be_bytes());
        bytes.extend_from_slice(&0x1234_5678u32.to_be_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(bank0);
        bytes.extend_from_slice(bank1);
        bytes
    }
}
