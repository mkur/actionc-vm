use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;

mod atr;
mod images;
mod memory;
mod object;
mod runner;

pub use atr::{AtrImage, BUNDLED_MYDOS_ATR, DiskWritePolicy, MountedDisk};
use images::{
    BUNDLED_ACTION_CARTRIDGE, BUNDLED_ACTION_CARTRIDGE_LABEL, BUNDLED_ALTIRRA_OS,
    BUNDLED_ALTIRRA_OS_LABEL, checksum16, crc32,
};
pub use images::{CarHeader, CartridgeMappingInfo, ImageKind, ImageMetadata, LoadedImage};
pub use memory::{AddressRange, Memory};
pub use object::{AtariLoadReport, AtariLoadSegment, load_atari_object_into_memory};
pub use runner::{
    PcTrigger, RunOutcome, RunReport, RunRequest, ScheduledAction, ScheduledActionObservation,
    ScheduledActions, StopReason, VmRunHooks, VmRunner,
};

pub const RAM_SIZE: usize = 0x10000;
pub const DEFAULT_CART_BASE: u16 = 0xA000;
pub const OS_ROM_BASE: u16 = 0xC000;
pub const IO_BASE: u16 = 0xD000;
pub const IO_SIZE: usize = 0x0800;
pub const SELF_TEST_BASE: u16 = 0x5000;
pub const SELF_TEST_SIZE: usize = 0x0800;
pub const BOOTQ_SUCCESSFUL_BOOT_FLAG: u16 = 0x0009;
pub const DOSVEC_START_VECTOR: u16 = 0x000A;
pub const DOSINI_INITIALIZATION_VECTOR: u16 = 0x000C;
pub const BRKKEY_BREAK_KEY_FLAG: u16 = 0x0011;
pub const PORTB: u16 = 0xD301;
pub const PACTL_PORTA_CONTROL: u16 = 0xD302;
pub const PBCTL_PORTB_CONTROL: u16 = 0xD303;
pub const PIA_DDR_ACCESS_DISABLE: u8 = 0x04;
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
pub const DEFAULT_HEADLESS_BRKKEY_NOT_PRESSED: u8 = 0x80;
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
pub const ACTION_CURRENT_PROC_POINTER: u16 = 0x008E;
pub const ACTION_SEGMENT_END_VECTOR: u16 = 0x04C6;
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
pub const HATABS_HANDLER_TABLE: u16 = 0x031A;
pub const CONSOL: u16 = 0xD01F;
pub const CONSOL_NO_KEYS: u8 = 0x07;
pub const SEROUT_SERIAL_OUTPUT: u16 = 0xD20D;
pub const CIOV: u16 = 0xE456;
pub const SIOV: u16 = 0xE459;
pub const DDEVIC: u16 = 0x0300;
pub const DUNIT: u16 = 0x0301;
pub const DCOMND: u16 = 0x0302;
pub const DSTATS: u16 = 0x0303;
pub const DBUFLO: u16 = 0x0304;
pub const DTIMLO: u16 = 0x0306;
pub const DBYTLO: u16 = 0x0308;
pub const DAUX1: u16 = 0x030A;
pub const SIO_DISK_DEVICE: u8 = 0x31;
pub const SIO_COMMAND_FORMAT: u8 = 0x21;
pub const SIO_COMMAND_FORMAT_ENHANCED: u8 = 0x22;
pub const SIO_COMMAND_PUT_SECTOR: u8 = 0x50;
pub const SIO_COMMAND_READ_SECTOR: u8 = 0x52;
pub const SIO_COMMAND_STATUS: u8 = 0x53;
pub const SIO_COMMAND_WRITE_SECTOR: u8 = 0x57;
pub const SIO_DIRECTION_READ: u8 = 0x40;
pub const SIO_DIRECTION_WRITE: u8 = 0x80;
pub const SIO_STATUS_SUCCESS: u8 = 0x01;
pub const SIO_STATUS_DEVICE_TIMEOUT: u8 = 0x8A;
pub const SIO_STATUS_DEVICE_NAK: u8 = 0x8B;
pub const SIO_STATUS_DEVICE_ERROR: u8 = 0x90;
pub const IOCB_DEVICE_BASE: u16 = 0x0341;
pub const IOCB_COMMAND_BASE: u16 = 0x0342;
pub const IOCB_BUFFER_BASE: u16 = 0x0344;
pub const IOCB_LENGTH_BASE: u16 = 0x0348;
pub const IOCB_AUX1_BASE: u16 = 0x034A;
pub const IOCB_AUX2_BASE: u16 = 0x034B;
pub const IOCB_AUX3_BASE: u16 = 0x034C;
pub const IOCB_AUX4_BASE: u16 = 0x034D;
pub const IOCB_AUX5_BASE: u16 = 0x034E;
pub const CIO_COMMAND_OPEN: u8 = 0x03;
pub const CIO_COMMAND_GETREC: u8 = 0x05;
pub const CIO_COMMAND_GETCHR: u8 = 0x07;
pub const CIO_COMMAND_PUTREC: u8 = 0x09;
pub const CIO_COMMAND_PUTCHR: u8 = 0x0B;
pub const CIO_COMMAND_CLOSE: u8 = 0x0C;
pub const CIO_COMMAND_STATUS: u8 = 0x0D;
pub const CIO_COMMAND_DRAW_TO: u8 = 0x11;
pub const CIO_COMMAND_FILL: u8 = 0x12;
pub const CIO_COMMAND_POINT: u8 = 0x25;
pub const CIO_COMMAND_NOTE: u8 = 0x26;
pub const ATASCII_EOL: u8 = 0x9B;
pub const GRAPHICS_COLOR: u16 = 0x02FD;
pub const GRAPHICS_FILL_COLOR: u16 = 0x02FB;
pub const CIO_OBSERVATION_LIMIT: usize = 128;
pub const CIO_READ_PREVIEW_LIMIT: usize = 80;
pub const SIO_OBSERVATION_LIMIT: usize = 128;
pub const RUNAD: u16 = 0x02E2;
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
pub enum ExecutionProfile {
    /// Boots and drives the original Action! cartridge compiler.
    OriginalCompiler,
    /// Runs an Atari object that still calls cartridge or OS services.
    CartridgeObject,
    /// Runs an Atari object whose emitted code is self-contained.
    StandaloneObject,
    /// Boots an Atari DOS from a mounted drive through the installed OS ROM.
    DiskBoot,
    /// Runs caller-installed memory and register state without external images.
    SyntheticTest,
}

/// Selects what the VM does when its headless CIO bridge does not own an IOCB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CioFallbackPolicy {
    /// Preserve the headless harness behavior, including accepting CLOSE on an
    /// otherwise empty channel.
    Headless,
    /// Let the installed Atari OS or DOS handler process unowned IOCBs.
    NativeOs,
}

impl ExecutionProfile {
    fn requires_cartridge_services(self) -> bool {
        matches!(self, Self::OriginalCompiler | Self::CartridgeObject)
    }

    fn requires_os_rom(self) -> bool {
        self.requires_cartridge_services() || self == Self::DiskBoot
    }
}

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
    pub disks: Vec<(u8, PathBuf, DiskWritePolicy)>,
    pub trace_cio: bool,
    pub trace_sio: bool,
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
            disks: Vec::new(),
            trace_cio: false,
            trace_sio: false,
        }
    }
}

impl VmConfig {
    pub fn apply_preset(&mut self, preset: MappingPreset) {
        self.cartridge_base = preset.cartridge_base;
        self.os_base = preset.os_base;
    }

    pub fn validate_for_execution(&self) -> Result<(), String> {
        self.validate_for_profile(ExecutionProfile::OriginalCompiler)
    }

    pub fn validate_for_profile(&self, profile: ExecutionProfile) -> Result<(), String> {
        if profile == ExecutionProfile::DiskBoot
            && !self.disks.iter().any(|(unit, _, _)| *unit == 1)
        {
            return Err("DiskBoot requires an ATR mounted on drive 1".to_string());
        }
        Ok(())
    }

    /// Loads only the images explicitly selected in this configuration.
    pub fn load(&self) -> Result<CompilerVm, String> {
        self.load_with_profile_defaults(None)
    }

    /// Loads the configuration and supplies the bundled Action! cartridge and
    /// AltirraOS images when the selected profile needs them and no overrides
    /// were given.
    pub fn load_for_profile(&self, profile: ExecutionProfile) -> Result<CompilerVm, String> {
        self.validate_for_profile(profile)?;
        self.load_with_profile_defaults(Some(profile))
    }

    fn load_with_profile_defaults(
        &self,
        profile: Option<ExecutionProfile>,
    ) -> Result<CompilerVm, String> {
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

        if profile.is_some_and(ExecutionProfile::requires_cartridge_services) {
            if vm.bus().cartridge().is_none() {
                vm.load_bundled_action_cartridge_at(self.cartridge_base)?;
            }
            if vm.bus().os_rom().is_none() {
                vm.load_bundled_altirra_os_at(self.os_base)?;
            }
        } else if profile.is_some_and(ExecutionProfile::requires_os_rom)
            && vm.bus().os_rom().is_none()
        {
            vm.load_bundled_altirra_os_at(self.os_base)?;
        }

        for hotpatch in &self.hotpatches {
            vm.apply_hotpatch(*hotpatch)?;
        }

        if let Some(path) = &self.source {
            let source = fs::read(path)
                .map_err(|err| format!("failed to read source `{}`: {err}", path.display()))?;
            vm.set_source_bytes(source);
        }

        for (name, path) in &self.host_files {
            let bytes = fs::read(path)
                .map_err(|err| format!("failed to read host file `{}`: {err}", path.display()))?;
            vm.add_host_file_bytes(name, bytes);
        }
        for (name, _) in &self.host_outputs {
            vm.add_host_output(name);
        }
        for (unit, path, policy) in &self.disks {
            let bytes = fs::read(path)
                .map_err(|err| format!("failed to read disk `{}`: {err}", path.display()))?;
            vm.mount_atr_bytes(*unit, bytes, *policy)?;
        }
        vm.set_trace_cio(self.trace_cio);
        vm.set_trace_sio(self.trace_sio);
        if let Some(profile) = profile {
            vm.apply_execution_profile_policies(profile);
            vm.validate_execution_profile(profile)?;
        }

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

    pub fn set_source_bytes(&mut self, source: impl Into<Vec<u8>>) {
        self.source = Some(source.into());
    }

    pub fn clear_source(&mut self) {
        self.source = None;
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

    pub fn validate_execution_profile(&self, profile: ExecutionProfile) -> Result<(), String> {
        if profile.requires_cartridge_services() && self.bus.cartridge().is_none() {
            return Err(format!("{profile:?} requires an Action! cartridge image"));
        }
        if profile.requires_cartridge_services() && self.bus.os_rom().is_none() {
            return Err(format!("{profile:?} requires an Atari OS ROM image"));
        }
        if profile == ExecutionProfile::DiskBoot && self.bus.os_rom().is_none() {
            return Err("DiskBoot requires an Atari OS ROM image".to_string());
        }
        if profile == ExecutionProfile::DiskBoot && self.bus.mounted_disk(1).is_none() {
            return Err("DiskBoot requires an ATR mounted on drive 1".to_string());
        }
        Ok(())
    }

    /// Installs the bundled Action! 3.6 cartridge at the standard address.
    ///
    /// A cartridge already installed by the caller is left unchanged.
    pub fn load_bundled_action_cartridge(&mut self) -> Result<(), String> {
        if self.bus.cartridge().is_none() {
            self.load_bundled_action_cartridge_at(DEFAULT_CART_BASE)?;
        }
        Ok(())
    }

    /// Installs the bundled AltirraOS XL/XE image at the standard OS address.
    ///
    /// An OS image already installed by the caller is left unchanged.
    pub fn load_bundled_altirra_os(&mut self) -> Result<(), String> {
        if self.bus.os_rom().is_none() {
            self.load_bundled_altirra_os_at(OS_ROM_BASE)?;
        }
        Ok(())
    }

    /// Ensures that the selected profile has its required cartridge and OS.
    /// Cartridge-backed profiles use the bundled Action! cartridge and
    /// AltirraOS unless the caller has already installed custom images.
    pub fn prepare_execution_profile(&mut self, profile: ExecutionProfile) -> Result<(), String> {
        if profile.requires_cartridge_services() {
            self.load_bundled_action_cartridge()?;
            self.load_bundled_altirra_os()?;
        } else if profile.requires_os_rom() {
            self.load_bundled_altirra_os()?;
        }
        self.apply_execution_profile_policies(profile);
        self.validate_execution_profile(profile)
    }

    fn apply_execution_profile_policies(&mut self, profile: ExecutionProfile) {
        let disk_boot = profile == ExecutionProfile::DiskBoot;
        self.bus.set_disk_boot_mode(disk_boot);
        self.bus.set_cio_fallback_policy(if disk_boot {
            CioFallbackPolicy::NativeOs
        } else {
            CioFallbackPolicy::Headless
        });
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.cpu.set_pc(pc);
    }

    pub fn prepare_headless_program_environment(&mut self) {
        self.bus.apply_headless_memory_defaults();
    }

    pub fn load_atari_object(&mut self, bytes: &[u8]) -> Result<AtariLoadReport, String> {
        load_atari_object_into_memory(self.bus.ram_mut(), bytes)
    }

    /// Prepares and starts an object under one of the object execution profiles.
    pub fn load_atari_object_for_execution(
        &mut self,
        profile: ExecutionProfile,
        bytes: &[u8],
    ) -> Result<AtariLoadReport, String> {
        if !matches!(
            profile,
            ExecutionProfile::CartridgeObject | ExecutionProfile::StandaloneObject
        ) {
            return Err(format!(
                "{profile:?} is not an Atari object execution profile"
            ));
        }
        self.prepare_execution_profile(profile)?;
        self.reset_cpu();
        self.prepare_headless_program_environment();
        let report = self.load_atari_object(bytes)?;
        let run_address = report
            .run_address
            .ok_or_else(|| "Atari object does not contain RUNAD".to_string())?;
        self.set_pc(run_address);
        Ok(report)
    }

    /// Loads an image supplied by a library caller without performing file I/O.
    ///
    /// `label` is retained only for diagnostics and image metadata; it need not
    /// identify a real filesystem path.
    pub fn load_image_bytes(
        &mut self,
        kind: ImageKind,
        label: impl Into<PathBuf>,
        base: u16,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        let image = LoadedImage::prepare(kind, label.into(), base, bytes)?;
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

    fn load_bundled_action_cartridge_at(&mut self, base: u16) -> Result<(), String> {
        self.load_image_bytes(
            ImageKind::Cartridge,
            BUNDLED_ACTION_CARTRIDGE_LABEL,
            base,
            BUNDLED_ACTION_CARTRIDGE.to_vec(),
        )
    }

    fn load_bundled_altirra_os_at(&mut self, base: u16) -> Result<(), String> {
        self.load_image_bytes(
            ImageKind::Rom,
            BUNDLED_ALTIRRA_OS_LABEL,
            base,
            BUNDLED_ALTIRRA_OS.to_vec(),
        )
    }

    pub fn add_host_file_bytes(&mut self, name: impl AsRef<str>, bytes: impl Into<Vec<u8>>) {
        self.bus.add_host_file(name, bytes.into());
    }

    pub fn add_host_output(&mut self, name: impl AsRef<str>) {
        self.bus.add_host_output(name);
    }

    pub fn host_file_bytes(&self, name: impl AsRef<str>) -> Option<&[u8]> {
        self.bus.host_file_bytes(name)
    }

    pub fn set_trace_cio(&mut self, trace_cio: bool) {
        self.bus.set_trace_cio(trace_cio);
    }

    pub fn set_trace_sio(&mut self, trace_sio: bool) {
        self.bus.set_trace_sio(trace_sio);
    }

    /// Mounts a validated 128-byte or 256-byte-sector ATR as drive 1 through 8.
    ///
    /// `ReadOnly` rejects sector writes and format requests. `CopyOnWrite`
    /// keeps the caller's input snapshot and applies changes only to the VM's
    /// in-memory image.
    pub fn mount_atr_bytes(
        &mut self,
        unit: u8,
        bytes: impl Into<Vec<u8>>,
        policy: DiskWritePolicy,
    ) -> Result<(), String> {
        self.bus.mount_atr_bytes(unit, bytes, policy)
    }

    /// Mounts the audited MyDOS 4.53/3 fixture on the requested drive.
    pub fn mount_bundled_mydos(&mut self, unit: u8, policy: DiskWritePolicy) -> Result<(), String> {
        self.mount_atr_bytes(unit, BUNDLED_MYDOS_ATR.to_vec(), policy)
    }

    /// Returns a serialized copy of the mounted image, including COW changes.
    pub fn mounted_atr_bytes(&self, unit: u8) -> Option<Vec<u8>> {
        self.bus.mounted_atr_bytes(unit)
    }

    /// Returns the unchanged ATR bytes supplied when the drive was mounted.
    pub fn original_atr_bytes(&self, unit: u8) -> Option<Vec<u8>> {
        self.bus.original_atr_bytes(unit)
    }

    /// Returns sorted, one-based logical sector numbers changed by COW writes.
    pub fn dirty_disk_sectors(&self, unit: u8) -> Option<Vec<u16>> {
        self.bus.dirty_disk_sectors(unit)
    }

    pub fn disk_is_dirty(&self, unit: u8) -> bool {
        self.bus.disk_is_dirty(unit)
    }

    pub fn protect_code_ranges(&mut self, ranges: &[AddressRange]) {
        self.bus.add_protected_code_ranges(ranges);
    }

    pub fn allow_code_write_ranges(&mut self, ranges: &[AddressRange]) {
        self.bus.add_allowed_code_write_ranges(ranges);
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
        self.load_image_bytes(kind, path, base, bytes)
    }
}

pub fn decode_action_symbol_tables(bus: &Bus) -> ActionSymbolTableDump {
    decode_action_symbol_tables_from_memory(bus.ram())
}

pub fn action_current_proc_name(bus: &Bus) -> Option<String> {
    action_current_proc_name_from_memory(bus.ram())
}

pub fn action_current_proc_name_from_memory(memory: &Memory) -> Option<String> {
    let address = memory.read_word(ACTION_CURRENT_PROC_POINTER);
    if address == 0 {
        return None;
    }
    let len = memory.read(address);
    if len == 0 {
        return None;
    }
    Some(decode_action_string_bytes(
        memory,
        address.wrapping_add(1),
        len,
    ))
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
        if pc == SIOV && self.try_emulate_siov(bus) {
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
            0x00 => {
                self.fetch_byte(bus);
                let return_address = self.registers.pc;
                self.push(bus, (return_address >> 8) as u8);
                self.push(bus, return_address as u8);
                self.push(
                    bus,
                    self.registers.status | StatusFlags::UNUSED.bits() | 0x10,
                );
                self.set_flag(StatusFlags::INTERRUPT_DISABLE, true);
                self.registers.pc = self.read_word(bus, 0xFFFE);
                self.cycles += 7;
            }
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
            0x16 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = value << 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 6;
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
            0x1E => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = value << 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 7;
            }
            0x20 => {
                let target = self.fetch_word(bus);
                let return_address = self.registers.pc.wrapping_sub(1);
                self.push(bus, (return_address >> 8) as u8);
                self.push(bus, return_address as u8);
                self.registers.pc = target;
                self.cycles += 6;
            }
            0x21 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 6;
            }
            0x24 => {
                let address = self.fetch_byte(bus) as u16;
                let value = bus.read(address);
                self.set_flag(StatusFlags::ZERO, self.registers.a & value == 0);
                self.set_flag(StatusFlags::NEGATIVE, value & 0x80 != 0);
                self.set_flag(StatusFlags::OVERFLOW, value & 0x40 != 0);
                self.cycles += 3;
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
            0x35 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x38 => {
                self.set_flag(StatusFlags::CARRY, true);
                self.cycles += 2;
            }
            0x39 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x3D => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.registers.a &= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x3E => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                let carry_in = u8::from(self.flag(StatusFlags::CARRY));
                self.set_flag(StatusFlags::CARRY, value & 0x80 != 0);
                let result = (value << 1) | carry_in;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 7;
            }
            0x40 => {
                self.registers.status = self.pop(bus) | StatusFlags::UNUSED.bits();
                let lo = self.pop(bus);
                let hi = self.pop(bus);
                self.registers.pc = u16::from_le_bytes([lo, hi]);
                self.cycles += 6;
            }
            0x41 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 6;
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
            0x50 => {
                self.branch(bus, !self.flag(StatusFlags::OVERFLOW), 2, 3);
            }
            0x51 => {
                let zp = self.fetch_byte(bus);
                let address = self.indirect_y(bus, zp);
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 5;
            }
            0x55 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x56 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
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
            0x59 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x5D => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.registers.a ^= value;
                self.set_zn(self.registers.a);
                self.cycles += 4;
            }
            0x5E => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.x as u16);
                let value = bus.read(address);
                self.set_flag(StatusFlags::CARRY, value & 0x01 != 0);
                let result = value >> 1;
                bus.write(address, result);
                self.set_zn(result);
                self.cycles += 7;
            }
            0x60 => {
                let lo = self.pop(bus);
                let hi = self.pop(bus);
                self.registers.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
                self.cycles += 6;
            }
            0x61 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.adc(value);
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
            0x70 => {
                self.branch(bus, self.flag(StatusFlags::OVERFLOW), 2, 3);
            }
            0x75 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 4;
            }
            0x76 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
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
            0x79 => {
                let base = self.fetch_word(bus);
                let address = base.wrapping_add(self.registers.y as u16);
                let value = bus.read(address);
                self.adc(value);
                self.cycles += 4;
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
            0x81 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                bus.write(address, self.registers.a);
                self.cycles += 6;
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
            0x94 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                bus.write(address, self.registers.y);
                self.cycles += 4;
            }
            0x95 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                bus.write(address, self.registers.a);
                self.cycles += 4;
            }
            0x96 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.y) as u16;
                bus.write(address, self.registers.x);
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
            0xA1 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 6;
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
            0xB4 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.registers.y = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xB5 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.registers.a = value;
                self.set_zn(value);
                self.cycles += 4;
            }
            0xB6 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.y) as u16;
                let value = bus.read(address);
                self.registers.x = value;
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
            0xC1 => {
                let zp = self.fetch_byte(bus);
                let address = self.indexed_indirect(bus, zp);
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 6;
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
            0xD5 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address);
                self.compare(self.registers.a, value);
                self.cycles += 4;
            }
            0xD6 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address).wrapping_sub(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 6;
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
            0xEC => {
                let address = self.fetch_word(bus);
                let value = bus.read(address);
                self.compare(self.registers.x, value);
                self.cycles += 4;
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
            0xF6 => {
                let base = self.fetch_byte(bus);
                let address = base.wrapping_add(self.registers.x) as u16;
                let value = bus.read(address).wrapping_add(1);
                bus.write(address, value);
                self.set_zn(value);
                self.cycles += 6;
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

        if let Some(write) = bus.take_protected_code_write() {
            self.halted = true;
            return Err(CpuError::ProtectedCodeWrite {
                pc,
                address: write.address,
                old_value: write.old_value,
                new_value: write.new_value,
                region: write.region,
            });
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
        let mut observation =
            bus.start_cio_observation(self.registers.x, command, return_pc, self.cycles);
        bus.trace_cio_call(&observation);
        match command {
            CIO_COMMAND_OPEN => {
                if bus.try_open_harness_cio_device(self.registers.x) {
                    observation.handled = true;
                    observation.detail = "open harness device".to_string();
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                observation.detail = "open passthrough".to_string();
                bus.finish_cio_observation(observation);
                false
            }
            CIO_COMMAND_GETCHR | CIO_COMMAND_GETREC => {
                match bus.cio_channel_device(self.registers.x) {
                    Some(CioHarnessDevice::QueuedInput) => {
                        if command == CIO_COMMAND_GETREC
                            && let Some(result) = bus.read_scripted_cio_record(self.registers.x)
                        {
                            observation.handled = true;
                            observation.detail = result.detail;
                            observation.result_a = Some(result.accumulator);
                            observation.result_y = Some(result.status);
                            observation.bytes_read = Some(result.bytes_read as u16);
                            if !result.preview.is_empty() {
                                observation.preview = Some(result.preview);
                            }
                            bus.finish_cio_observation(observation);
                            self.return_from_ciov(bus, result.accumulator, result.status);
                            return true;
                        }
                        if command == CIO_COMMAND_GETCHR
                            && let Some(character) = bus.pop_scripted_cio_input_byte()
                        {
                            bus.trace_cio(format_args!(
                                "  Q: read ${character:02X} `{}`",
                                atari_debug_char(character)
                            ));
                            observation.handled = true;
                            observation.detail = format!("read queued input ${character:02X}");
                            observation.result_a = Some(character);
                            observation.result_y = Some(0x01);
                            observation.bytes_read = Some(1);
                            observation.preview = Some(format_cio_preview(&[character]));
                            bus.finish_cio_observation(observation);
                            self.return_from_ciov(bus, character, 0x01);
                            return true;
                        }
                    }
                    Some(CioHarnessDevice::Host { .. }) => {
                        let result = if command == CIO_COMMAND_GETREC {
                            bus.read_host_record(self.registers.x)
                        } else if bus
                            .ram()
                            .read_word(IOCB_LENGTH_BASE.wrapping_add(self.registers.x as u16))
                            != 0
                        {
                            bus.read_host_block(self.registers.x)
                        } else {
                            bus.read_host_character(self.registers.x)
                        };
                        if let Some(result) = result {
                            observation.handled = true;
                            observation.detail = result.detail;
                            observation.result_a = Some(result.accumulator);
                            observation.result_y = Some(result.status);
                            observation.bytes_read = Some(result.bytes_read as u16);
                            if !result.preview.is_empty() {
                                observation.preview = Some(result.preview);
                            }
                            bus.finish_cio_observation(observation);
                            let accumulator = result.accumulator;
                            let status = result.status;
                            self.return_from_ciov(bus, accumulator, status);
                            return true;
                        }
                    }
                    Some(CioHarnessDevice::Screen) => {
                        let character = bus.read_screen_pixel();
                        observation.handled = true;
                        observation.detail = format!("read screen pixel ${character:02X}");
                        observation.result_a = Some(character);
                        observation.result_y = Some(0x01);
                        observation.bytes_read = Some(1);
                        observation.preview = Some(format_cio_preview(&[character]));
                        bus.finish_cio_observation(observation);
                        self.return_from_ciov(bus, character, 0x01);
                        return true;
                    }
                    Some(CioHarnessDevice::Editor) => {}
                    None => {}
                }

                if self.registers.x != 0x70 {
                    observation.detail = "read passthrough".to_string();
                    bus.finish_cio_observation(observation);
                    return false;
                }
                let raw_key = bus.ram().read(CH_KEY_CODE);
                let Some(character) = atari_key_code_to_character(raw_key) else {
                    bus.keyboard_read_waiting = true;
                    observation.detail = "keyboard read waiting".to_string();
                    bus.finish_cio_observation(observation);
                    return false;
                };

                bus.keyboard_read_waiting = false;
                bus.write(CH_KEY_CODE, 0xFF);
                observation.handled = true;
                observation.detail = format!("keyboard read ${character:02X}");
                observation.result_a = Some(character);
                observation.result_y = Some(0x01);
                observation.bytes_read = Some(1);
                observation.preview = Some(format_cio_preview(&[character]));
                bus.finish_cio_observation(observation);
                self.return_from_ciov(bus, character, 0x01);
                true
            }
            CIO_COMMAND_CLOSE => {
                let closed_harness_device = bus.close_harness_cio_device(self.registers.x);
                if !closed_harness_device
                    && bus.cio_fallback_policy() == CioFallbackPolicy::NativeOs
                {
                    observation.detail = "close passthrough".to_string();
                    bus.finish_cio_observation(observation);
                    return false;
                }
                observation.handled = true;
                observation.detail = if closed_harness_device {
                    "close harness device".to_string()
                } else {
                    "close empty channel".to_string()
                };
                observation.result_a = Some(self.registers.a);
                observation.result_y = Some(0x01);
                bus.finish_cio_observation(observation);
                self.return_from_ciov(bus, self.registers.a, 0x01);
                true
            }
            CIO_COMMAND_STATUS => {
                if bus.cio_channel_device(self.registers.x).is_some() {
                    observation.handled = true;
                    observation.detail = "status harness device".to_string();
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                observation.detail = "status passthrough".to_string();
                bus.finish_cio_observation(observation);
                false
            }
            CIO_COMMAND_NOTE => {
                if let Some(offset) = bus.note_host_position(self.registers.x) {
                    observation.handled = true;
                    observation.detail = format!("note host offset {offset}");
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                observation.detail = "note passthrough".to_string();
                bus.finish_cio_observation(observation);
                false
            }
            CIO_COMMAND_POINT => {
                if let Some(offset) = bus.point_host_position(self.registers.x) {
                    observation.handled = true;
                    observation.detail = format!("point host offset {offset}");
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                observation.detail = "point passthrough".to_string();
                bus.finish_cio_observation(observation);
                false
            }
            CIO_COMMAND_DRAW_TO | CIO_COMMAND_FILL => {
                if bus.draw_screen_to_cursor(self.registers.x, command == CIO_COMMAND_FILL) {
                    observation.handled = true;
                    observation.detail = if command == CIO_COMMAND_FILL {
                        "fill screen rectangle".to_string()
                    } else {
                        "draw screen line".to_string()
                    };
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                observation.detail = format!("command ${command:02X} passthrough");
                bus.finish_cio_observation(observation);
                false
            }
            CIO_COMMAND_PUTCHR | CIO_COMMAND_PUTREC => {
                let terminate_record = command == CIO_COMMAND_PUTREC;
                if let Some(written) = bus.write_screen_bytes_for_iocb(
                    self.registers.x,
                    self.registers.a,
                    terminate_record,
                ) {
                    observation.handled = true;
                    observation.detail = format!("write screen {written} pixel(s)");
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    observation.bytes_written = Some(written as u16);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                if let Some(written) = bus.write_host_bytes_for_iocb(
                    self.registers.x,
                    self.registers.a,
                    terminate_record,
                ) {
                    observation.handled = true;
                    observation.detail = format!("write host {written} byte(s)");
                    observation.result_a = Some(self.registers.a);
                    observation.result_y = Some(0x01);
                    observation.bytes_written = Some(written as u16);
                    bus.finish_cio_observation(observation);
                    self.return_from_ciov(bus, self.registers.a, 0x01);
                    return true;
                }
                if self.registers.x != 0x00 {
                    observation.detail = "write passthrough".to_string();
                    bus.finish_cio_observation(observation);
                    return false;
                }
                let bytes = bus.cio_output_bytes_for_iocb(
                    self.registers.x,
                    self.registers.a,
                    terminate_record,
                );
                bus.capture_cio_channel0_output(&bytes);
                observation.handled = true;
                observation.detail = format!("write E: {} byte(s)", bytes.len());
                observation.result_a = Some(self.registers.a);
                observation.result_y = Some(0x01);
                observation.bytes_written = Some(bytes.len() as u16);
                bus.finish_cio_observation(observation);
                self.return_from_ciov(bus, self.registers.a, 0x01);
                true
            }
            _ => {
                observation.detail = format!("command ${command:02X} passthrough");
                bus.finish_cio_observation(observation);
                false
            }
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

    fn try_emulate_siov(&mut self, bus: &mut Bus) -> bool {
        let return_pc = self.peek_return_address(bus);
        let Some(status) = bus.try_service_siov(return_pc, self.cycles) else {
            return false;
        };

        // The OS SIO exit leaves A=0, returns status in Y and DSTATS, and
        // performs CPY #0 before RTS. Thus carry is set for every byte-sized
        // status and N reflects the high bit used by SIO errors.
        self.registers.a = 0;
        self.registers.y = status;
        let lo = self.pop(bus);
        let hi = self.pop(bus);
        self.registers.pc = u16::from_le_bytes([lo, hi]).wrapping_add(1);
        self.set_flag(StatusFlags::CARRY, true);
        self.set_zn(status);
        self.cycles += 6;
        true
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

        if self.flag(StatusFlags::DECIMAL) {
            // The Atari uses an NMOS 6502. Its decimal result is BCD-adjusted,
            // while N, V, and Z retain the original chip's intermediate/binary
            // behavior rather than describing the final adjusted byte.
            let binary_result = lhs.wrapping_add(value).wrapping_add(carry);
            let mut low_nibble = (lhs & 0x0F) + (value & 0x0F) + carry;
            let low_carry = low_nibble > 9;
            if low_carry {
                low_nibble = low_nibble.wrapping_sub(10) & 0x0F;
            }

            let mut high_nibble = (lhs >> 4) + (value >> 4) + u8::from(low_carry);
            let negative = high_nibble & 0x08 != 0;
            let overflow = ((lhs >= 0x80) ^ negative) && ((value >= 0x80) ^ negative);
            let high_carry = high_nibble > 9;
            if high_carry {
                high_nibble = high_nibble.wrapping_sub(10) & 0x0F;
            }

            self.registers.a = (high_nibble << 4) | low_nibble;
            self.set_flag(StatusFlags::NEGATIVE, negative);
            self.set_flag(StatusFlags::OVERFLOW, overflow);
            self.set_flag(StatusFlags::ZERO, binary_result == 0);
            self.set_flag(StatusFlags::CARRY, high_carry);
            return;
        }

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

        if self.flag(StatusFlags::DECIMAL) {
            // NMOS decimal subtraction reports N, V, and Z from the binary
            // subtraction even though the accumulator receives a BCD result.
            let binary_result = lhs.wrapping_sub(value).wrapping_sub(borrow);
            let negative = binary_result & 0x80 != 0;
            let overflow = ((lhs >= 0x80) ^ negative) && ((value < 0x80) ^ negative);

            let mut low_nibble = (lhs & 0x0F).wrapping_sub(value & 0x0F).wrapping_sub(borrow);
            let low_borrow = low_nibble >= 0x80;
            if low_borrow {
                low_nibble = low_nibble.wrapping_add(10) & 0x0F;
            }

            let mut high_nibble = (lhs >> 4)
                .wrapping_sub(value >> 4)
                .wrapping_sub(u8::from(low_borrow));
            let high_borrow = high_nibble >= 0x80;
            if high_borrow {
                high_nibble = high_nibble.wrapping_add(10) & 0x0F;
            }

            self.registers.a = (high_nibble << 4) | low_nibble;
            self.set_flag(StatusFlags::NEGATIVE, negative);
            self.set_flag(StatusFlags::OVERFLOW, overflow);
            self.set_flag(StatusFlags::ZERO, binary_result == 0);
            self.set_flag(StatusFlags::CARRY, !high_borrow);
            return;
        }

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
    UnsupportedOpcode {
        pc: u16,
        opcode: u8,
    },
    ProtectedCodeWrite {
        pc: u16,
        address: u16,
        old_value: u8,
        new_value: u8,
        region: BusRegion,
    },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bus {
    ram: Memory,
    io: IoRegion,
    os_rom: Option<RomRegion>,
    cartridge: Option<Cartridge>,
    watchpoints: Vec<AddressRange>,
    protected_code_ranges: Vec<AddressRange>,
    allowed_code_write_ranges: Vec<AddressRange>,
    protected_code_write: Option<ProtectedCodeWrite>,
    events: Vec<BusEvent>,
    last_data: u8,
    vcount: u8,
    pending_key_codes: VecDeque<u8>,
    scripted_cio_input: VecDeque<u8>,
    scripted_cio_input_was_queued: bool,
    keyboard_read_waiting: bool,
    cio_channel0_output: Vec<u8>,
    cio_harness_devices: [Option<CioHarnessDevice>; 8],
    graphics_pixels: HashMap<(u16, u8), u8>,
    graphics_pen: Option<(u16, u8)>,
    graphics_mode: Option<u8>,
    host_files: Vec<HostFile>,
    host_file_lookup: HashMap<String, usize>,
    trace_cio: bool,
    cio_summary: CioSummary,
    cio_observations: VecDeque<CioObservation>,
    cio_fallback_policy: CioFallbackPolicy,
    mounted_disks: [Option<MountedDisk>; 8],
    trace_sio: bool,
    sio_summary: SioSummary,
    sio_observations: VecDeque<SioObservation>,
    sio_timeout_pending: bool,
    redirect_disk_boot_to_cart: bool,
    disk_boot_mode: bool,
    self_test_rom_enabled: bool,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            ram: Memory::default(),
            io: IoRegion::default(),
            os_rom: None,
            cartridge: None,
            watchpoints: Vec::new(),
            protected_code_ranges: Vec::new(),
            allowed_code_write_ranges: Vec::new(),
            protected_code_write: None,
            events: Vec::new(),
            last_data: 0,
            vcount: 0,
            pending_key_codes: VecDeque::new(),
            scripted_cio_input: VecDeque::new(),
            scripted_cio_input_was_queued: false,
            keyboard_read_waiting: false,
            cio_channel0_output: Vec::new(),
            cio_harness_devices: [None; 8],
            graphics_pixels: HashMap::new(),
            graphics_pen: None,
            graphics_mode: None,
            host_files: Vec::new(),
            host_file_lookup: HashMap::new(),
            trace_cio: false,
            cio_summary: CioSummary::default(),
            cio_observations: VecDeque::new(),
            cio_fallback_policy: CioFallbackPolicy::Headless,
            mounted_disks: std::array::from_fn(|_| None),
            trace_sio: false,
            sio_summary: SioSummary::default(),
            sio_observations: VecDeque::new(),
            sio_timeout_pending: false,
            redirect_disk_boot_to_cart: false,
            disk_boot_mode: false,
            self_test_rom_enabled: true,
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

    pub fn add_protected_code_range(&mut self, range: AddressRange) {
        if !self.protected_code_ranges.contains(&range) {
            self.protected_code_ranges.push(range);
        }
    }

    pub fn add_protected_code_ranges(&mut self, ranges: &[AddressRange]) {
        for range in ranges {
            self.add_protected_code_range(*range);
        }
    }

    pub fn protected_code_ranges(&self) -> &[AddressRange] {
        &self.protected_code_ranges
    }

    pub fn add_allowed_code_write_range(&mut self, range: AddressRange) {
        if !self.allowed_code_write_ranges.contains(&range) {
            self.allowed_code_write_ranges.push(range);
        }
    }

    pub fn add_allowed_code_write_ranges(&mut self, ranges: &[AddressRange]) {
        for range in ranges {
            self.add_allowed_code_write_range(*range);
        }
    }

    pub fn take_protected_code_write(&mut self) -> Option<ProtectedCodeWrite> {
        self.protected_code_write.take()
    }

    pub fn events(&self) -> &[BusEvent] {
        &self.events
    }

    pub fn cio_summary(&self) -> &CioSummary {
        &self.cio_summary
    }

    pub fn cio_observations(&self) -> &VecDeque<CioObservation> {
        &self.cio_observations
    }

    pub fn sio_observations(&self) -> &VecDeque<SioObservation> {
        &self.sio_observations
    }

    pub fn sio_summary(&self) -> &SioSummary {
        &self.sio_summary
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn queue_key_code(&mut self, key_code: u8) {
        self.keyboard_read_waiting = false;
        if self.ram.read(CH_KEY_CODE) == 0xFF {
            self.deliver_key_code(key_code);
        } else {
            self.pending_key_codes.push_back(key_code);
        }
    }

    pub fn queue_scripted_cio_input_byte(&mut self, byte: u8) {
        self.scripted_cio_input_was_queued = true;
        self.keyboard_read_waiting = false;
        self.scripted_cio_input.push_back(byte);
    }

    pub fn queue_scripted_cio_input_bytes(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.scripted_cio_input_was_queued = true;
        self.keyboard_read_waiting = false;
        self.scripted_cio_input.extend(bytes);
    }

    /// True after scripted CIO input has been consumed and the VM has observed
    /// the program waiting for another channel-7 keyboard character.
    pub fn scripted_cio_input_is_idle(&self) -> bool {
        self.scripted_cio_input_was_queued
            && self.scripted_cio_input.is_empty()
            && self.pending_key_codes.is_empty()
            && self.ram.read(CH_KEY_CODE) == 0xFF
            && self.keyboard_read_waiting
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

    pub fn set_trace_sio(&mut self, trace_sio: bool) {
        self.trace_sio = trace_sio;
    }

    pub fn mount_atr_bytes(
        &mut self,
        unit: u8,
        bytes: impl Into<Vec<u8>>,
        policy: DiskWritePolicy,
    ) -> Result<(), String> {
        let index = disk_unit_index(unit)?;
        let image = AtrImage::from_bytes(bytes)?;
        self.mounted_disks[index] = Some(MountedDisk {
            unit,
            image,
            write_policy: policy,
        });
        Ok(())
    }

    pub fn unmount_disk(&mut self, unit: u8) -> Result<Option<MountedDisk>, String> {
        let index = disk_unit_index(unit)?;
        Ok(self.mounted_disks[index].take())
    }

    pub fn mounted_disk(&self, unit: u8) -> Option<&MountedDisk> {
        let index = disk_unit_index(unit).ok()?;
        self.mounted_disks[index].as_ref()
    }

    fn mounted_disk_mut(&mut self, unit: u8) -> Option<&mut MountedDisk> {
        let index = disk_unit_index(unit).ok()?;
        self.mounted_disks[index].as_mut()
    }

    pub fn mounted_atr_bytes(&self, unit: u8) -> Option<Vec<u8>> {
        Some(self.mounted_disk(unit)?.image.as_bytes().to_vec())
    }

    pub fn original_atr_bytes(&self, unit: u8) -> Option<Vec<u8>> {
        Some(self.mounted_disk(unit)?.image.original_bytes().to_vec())
    }

    pub fn dirty_disk_sectors(&self, unit: u8) -> Option<Vec<u16>> {
        Some(self.mounted_disk(unit)?.image.dirty_sectors())
    }

    pub fn disk_is_dirty(&self, unit: u8) -> bool {
        self.mounted_disk(unit)
            .is_some_and(|disk| disk.image.is_dirty())
    }

    pub fn set_cio_fallback_policy(&mut self, policy: CioFallbackPolicy) {
        self.cio_fallback_policy = policy;
    }

    pub fn cio_fallback_policy(&self) -> CioFallbackPolicy {
        self.cio_fallback_policy
    }

    pub fn disk_boot_mode(&self) -> bool {
        self.disk_boot_mode
    }

    pub fn cio_handler_address(&self, device: u8) -> Option<u16> {
        for index in 0..12u16 {
            let entry = HATABS_HANDLER_TABLE.wrapping_add(index * 3);
            let entry_device = self.ram.read(entry);
            if entry_device == 0 {
                return None;
            }
            if entry_device == device {
                let address = self.ram.read_word(entry.wrapping_add(1));
                return (address != 0).then_some(address);
            }
        }
        None
    }

    /// Reports that a disk boot completed far enough to publish DOS vectors
    /// and install a native `D:` handler in HATABS.
    pub fn dos_boot_is_ready(&self) -> bool {
        self.disk_boot_mode
            && self.ram.read(BOOTQ_SUCCESSFUL_BOOT_FLAG) == 1
            && self.ram.read_word(DOSVEC_START_VECTOR) != 0
            && self.ram.read_word(DOSINI_INITIALIZATION_VECTOR) != 0
            && self.cio_handler_address(b'D').is_some()
    }

    fn set_disk_boot_mode(&mut self, enabled: bool) {
        self.disk_boot_mode = enabled;
        if enabled {
            self.sio_timeout_pending = false;
            self.redirect_disk_boot_to_cart = false;
        }
    }

    pub fn cio_channel0_output(&self) -> &[u8] {
        &self.cio_channel0_output
    }

    pub fn graphics_pixel(&self, column: u16, row: u8) -> u8 {
        self.graphics_pixels
            .get(&(column, row))
            .copied()
            .unwrap_or(0)
    }

    pub fn graphics_mode(&self) -> Option<u8> {
        self.graphics_mode
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

    pub fn visible_region(&self, address: u16) -> BusRegion {
        if let Some(cartridge) = self.cartridge.as_ref() {
            if cartridge.read(address).is_some() {
                return BusRegion::Cartridge;
            }
        }
        if self.io.contains(address) {
            return BusRegion::Io;
        }
        if self.read_self_test(address).is_some() {
            return BusRegion::SelfTestRom;
        }
        if self
            .os_rom
            .as_ref()
            .is_some_and(|os_rom| os_rom.contains(address))
        {
            return BusRegion::OsRom;
        }
        BusRegion::Ram
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
                if self.protect_ram_write(address, value) {
                    BusRegion::Ram
                } else {
                    self.ram.write(address, value);
                    BusRegion::Ram
                }
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
            if self.protect_ram_write(address, value) {
                BusRegion::Ram
            } else {
                self.ram.write(address, value);
                BusRegion::Ram
            }
        };

        if address == SEROUT_SERIAL_OUTPUT {
            self.ram.write(XMTDON_TRANSMISSION_DONE_FLAG, 0xFF);
            self.ram.write(RECVDN_RECEIVE_DONE_FLAG, 0x00);
            self.sio_timeout_pending = true;
            if self.disk_boot_mode {
                self.sio_timeout_pending = false;
            }
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

    fn protect_ram_write(&mut self, address: u16, value: u8) -> bool {
        if !self
            .protected_code_ranges
            .iter()
            .any(|range| range.contains(address))
            || self
                .allowed_code_write_ranges
                .iter()
                .any(|range| range.contains(address))
        {
            return false;
        }
        if self.protected_code_write.is_none() {
            self.protected_code_write = Some(ProtectedCodeWrite {
                address,
                old_value: self.ram.read(address),
                new_value: value,
                region: BusRegion::Ram,
            });
        }
        true
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
        self.io.disable_self_test_rom();
        self.self_test_rom_enabled = false;
        let [lo, hi] = target.to_le_bytes();
        self.ram.write(BOOTQ_SUCCESSFUL_BOOT_FLAG, 0x01);
        self.ram.write(DOSVEC_START_VECTOR, lo);
        self.ram.write(DOSVEC_START_VECTOR.wrapping_add(1), hi);
    }

    fn apply_headless_memory_defaults(&mut self) {
        self.ram
            .write(BRKKEY_BREAK_KEY_FLAG, DEFAULT_HEADLESS_BRKKEY_NOT_PRESSED);
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
        let byte = self.scripted_cio_input.pop_front();
        if byte.is_some() {
            self.keyboard_read_waiting = false;
        }
        byte
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
            b'E' => CioHarnessDevice::Editor,
            b'S' => {
                self.graphics_mode = Some(self.ram.read(IOCB_AUX1_BASE.wrapping_add(x as u16)));
                self.graphics_pixels.clear();
                self.graphics_pen = None;
                CioHarnessDevice::Screen
            }
            b'H' | b'D' => {
                let device_letter = self.peek_mapped(spec_buffer).to_ascii_uppercase();
                // Once DOS has been booted, its installed D: handler owns all
                // disk semantics. H: remains available for explicit host files.
                if self.disk_boot_mode && device_letter == b'D' {
                    return false;
                }
                let device_name = device_letter as char;
                let spec = self.read_iocb_string(spec_buffer, spec_length);
                let name = normalize_host_file_name(&spec);
                let Some(file_index) = self.host_file_lookup.get(&name).copied() else {
                    self.trace_cio(format_args!(
                        "  {device_name}: open miss spec=`{spec}` name=`{name}`"
                    ));
                    return false;
                };
                self.trace_cio(format_args!(
                    "  {device_name}: open spec=`{spec}` name=`{name}`"
                ));
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
        self.trace_cio(format_args!(
            "  harness open channel={channel} device={device:?}"
        ));
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
        if length > 0 && first == length as u8 && self.peek_mapped(buffer.wrapping_add(2)) == b':' {
            (buffer.wrapping_add(1), length)
        } else {
            (buffer, length)
        }
    }

    fn peek_mapped(&mut self, address: u16) -> u8 {
        self.read(address)
    }

    fn read_host_character(&mut self, x: u8) -> Option<CioReadResult> {
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
            let output = host_source_byte_to_atascii(byte);
            return Some(CioReadResult {
                accumulator: output,
                status: 0x01,
                bytes_read: 1,
                detail: format!("read host char ${output:02X}"),
                preview: format_cio_preview(&[output]),
            });
        }
        Some(CioReadResult {
            accumulator: 0x88,
            status: 0x88,
            bytes_read: 0,
            detail: "read host char EOF".to_string(),
            preview: String::new(),
        })
    }

    fn read_host_block(&mut self, x: u8) -> Option<CioReadResult> {
        let channel = cio_channel_index(x)?;
        let Some(CioHarnessDevice::Host { file_index, offset }) = self.cio_harness_devices[channel]
        else {
            return None;
        };
        let requested = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(x as u16));
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(x as u16));
        let file = self.host_files.get(file_index)?;
        if file.writable || requested == 0 {
            return None;
        }

        let available = file.bytes.len().saturating_sub(offset);
        let transferred = usize::from(requested).min(available);
        let mut preview = Vec::new();
        for index in 0..transferred {
            let byte = file.bytes[offset + index];
            self.ram.write(buffer.wrapping_add(index as u16), byte);
            if preview.len() < CIO_READ_PREVIEW_LIMIT {
                preview.push(byte);
            }
        }

        self.ram
            .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), transferred as u16);
        self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host {
            file_index,
            offset: offset + transferred,
        });

        let status = if transferred == usize::from(requested) {
            0x01
        } else {
            0x88
        };
        Some(CioReadResult {
            accumulator: if status == 0x01 { 0 } else { status },
            status,
            bytes_read: transferred,
            detail: if status == 0x01 {
                format!("read host block {transferred} byte(s)")
            } else {
                format!("read host block {transferred} byte(s), EOF")
            },
            preview: format_cio_preview(&preview),
        })
    }

    fn read_scripted_cio_record(&mut self, x: u8) -> Option<CioReadResult> {
        let requested = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(x as u16));
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(x as u16));
        if requested == 0 || self.scripted_cio_input.is_empty() {
            return None;
        }

        let mut written = 0u16;
        let mut preview = Vec::new();
        while written < requested {
            let Some(byte) = self.pop_scripted_cio_input_byte() else {
                break;
            };
            self.ram.write(buffer.wrapping_add(written), byte);
            if preview.len() < CIO_READ_PREVIEW_LIMIT {
                preview.push(byte);
            }
            written = written.wrapping_add(1);
            if byte == 0x9B {
                break;
            }
        }

        self.ram
            .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), written);
        Some(CioReadResult {
            accumulator: 0,
            status: 0x01,
            bytes_read: written as usize,
            detail: format!("read queued record {written} byte(s)"),
            preview: format_cio_preview(&preview),
        })
    }

    fn read_host_record(&mut self, x: u8) -> Option<CioReadResult> {
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
            return Some(CioReadResult {
                accumulator: 0x88,
                status: 0x88,
                bytes_read: 0,
                detail: "read host record EOF".to_string(),
                preview: String::new(),
            });
        }

        let mut next_offset = offset;
        let mut written = 0u16;
        let mut wrote_eol = false;
        let mut preview = Vec::new();
        while written < requested && next_offset < file.bytes.len() {
            let byte = file.bytes[next_offset];
            next_offset += 1;
            if byte == b'\r' {
                continue;
            }
            let output = host_source_byte_to_atascii(byte);
            self.ram.write(buffer.wrapping_add(written), output);
            if preview.len() < CIO_READ_PREVIEW_LIMIT {
                preview.push(output);
            }
            written = written.wrapping_add(1);
            if output == 0x9B {
                wrote_eol = true;
                break;
            }
        }

        if written == 0 {
            self.ram
                .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), 0);
            return Some(CioReadResult {
                accumulator: 0x88,
                status: 0x88,
                bytes_read: 0,
                detail: "read host record EOF".to_string(),
                preview: String::new(),
            });
        }

        if !wrote_eol && written < requested {
            self.ram.write(buffer.wrapping_add(written), 0x9B);
            if preview.len() < CIO_READ_PREVIEW_LIMIT {
                preview.push(0x9B);
            }
            written = written.wrapping_add(1);
        }

        self.ram
            .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), written);
        self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host {
            file_index,
            offset: next_offset,
        });
        Some(CioReadResult {
            accumulator: 0,
            status: 0x01,
            bytes_read: written as usize,
            detail: format!("read host record {written} byte(s)"),
            preview: format_cio_preview(&preview),
        })
    }

    fn note_host_position(&mut self, x: u8) -> Option<usize> {
        let CioHarnessDevice::Host { offset, .. } = self.cio_channel_device(x)? else {
            return None;
        };
        let sector = offset / 256;
        self.ram
            .write(IOCB_AUX3_BASE.wrapping_add(x as u16), (sector & 0xFF) as u8);
        self.ram.write(
            IOCB_AUX4_BASE.wrapping_add(x as u16),
            ((sector >> 8) & 0xFF) as u8,
        );
        self.ram
            .write(IOCB_AUX5_BASE.wrapping_add(x as u16), (offset & 0xFF) as u8);
        Some(offset)
    }

    fn point_host_position(&mut self, x: u8) -> Option<usize> {
        let channel = cio_channel_index(x)?;
        let CioHarnessDevice::Host { file_index, .. } = self.cio_harness_devices[channel]? else {
            return None;
        };
        let sector = u16::from_le_bytes([
            self.ram.read(IOCB_AUX3_BASE.wrapping_add(x as u16)),
            self.ram.read(IOCB_AUX4_BASE.wrapping_add(x as u16)),
        ]);
        let byte = self.ram.read(IOCB_AUX5_BASE.wrapping_add(x as u16));
        let offset = usize::from(sector)
            .saturating_mul(256)
            .saturating_add(usize::from(byte));
        if offset > self.host_files.get(file_index)?.bytes.len() {
            return None;
        }
        self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host { file_index, offset });
        Some(offset)
    }

    fn write_host_bytes_for_iocb(
        &mut self,
        x: u8,
        accumulator: u8,
        terminate_record: bool,
    ) -> Option<usize> {
        let channel = cio_channel_index(x)?;
        let Some(CioHarnessDevice::Host { file_index, offset }) = self.cio_harness_devices[channel]
        else {
            return None;
        };
        if !self.host_files.get(file_index)?.writable {
            return None;
        }

        let bytes = self.cio_output_bytes_for_iocb(x, accumulator, terminate_record);
        self.host_files[file_index].bytes.extend_from_slice(&bytes);
        self.cio_harness_devices[channel] = Some(CioHarnessDevice::Host {
            file_index,
            offset: offset.saturating_add(bytes.len()),
        });
        self.trace_cio(format_args!(
            "  host wrote {} byte(s) to `{}`",
            bytes.len(),
            self.host_files[file_index].name
        ));
        Some(bytes.len())
    }

    fn read_screen_pixel(&self) -> u8 {
        self.graphics_pixel(self.ram.read_word(COLCRS), self.ram.read(ROWCRS))
    }

    fn write_screen_bytes_for_iocb(
        &mut self,
        x: u8,
        accumulator: u8,
        terminate_record: bool,
    ) -> Option<usize> {
        if self.cio_channel_device(x) != Some(CioHarnessDevice::Screen) {
            return None;
        }
        let bytes = self.cio_output_bytes_for_iocb(x, accumulator, terminate_record);
        let mut column = self.ram.read_word(COLCRS);
        let row = self.ram.read(ROWCRS);
        for byte in &bytes {
            self.graphics_pixels.insert((column, row), *byte);
            self.graphics_pen = Some((column, row));
            column = column.wrapping_add(1);
        }
        Some(bytes.len())
    }

    fn draw_screen_to_cursor(&mut self, x: u8, fill: bool) -> bool {
        if self.cio_channel_device(x) != Some(CioHarnessDevice::Screen) {
            return false;
        }
        let target = (self.ram.read_word(COLCRS), self.ram.read(ROWCRS));
        let start = self.graphics_pen.unwrap_or(target);
        let color = self.ram.read(GRAPHICS_FILL_COLOR);
        if fill {
            let min_column = start.0.min(target.0);
            let max_column = start.0.max(target.0);
            let min_row = start.1.min(target.1);
            let max_row = start.1.max(target.1);
            for row in min_row..=max_row {
                for column in min_column..=max_column {
                    self.graphics_pixels.insert((column, row), color);
                }
            }
        } else {
            self.draw_screen_line(start, target, color);
        }
        self.graphics_pen = Some(target);
        true
    }

    fn draw_screen_line(&mut self, start: (u16, u8), end: (u16, u8), color: u8) {
        let (mut x0, mut y0) = (i32::from(start.0), i32::from(start.1));
        let (x1, y1) = (i32::from(end.0), i32::from(end.1));
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.graphics_pixels.insert((x0 as u16, y0 as u8), color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice_error = error * 2;
            if twice_error >= dy {
                error += dy;
                x0 += sx;
            }
            if twice_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn cio_output_bytes_for_iocb(&self, x: u8, accumulator: u8, terminate_record: bool) -> Vec<u8> {
        let base = x as u16;
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(base));
        let length = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(base));
        if buffer == 0 || length == 0 {
            let mut bytes = vec![accumulator];
            if terminate_record && accumulator != ATASCII_EOL {
                bytes.push(ATASCII_EOL);
            }
            return bytes;
        }

        let mut bytes = Vec::with_capacity(length as usize);
        for offset in 0..length {
            bytes.push(self.ram.read(buffer.wrapping_add(offset)));
        }
        if terminate_record && bytes.last() != Some(&ATASCII_EOL) {
            bytes.push(ATASCII_EOL);
        }
        bytes
    }

    fn capture_cio_channel0_output(&mut self, bytes: &[u8]) {
        self.ensure_text_cursor_defaults();
        self.cio_channel0_output.extend(bytes);
        for byte in bytes {
            if *byte == 0x9B {
                self.ram.write(COLCRS, 0);
                self.ram
                    .write(ROWCRS, self.ram.read(ROWCRS).wrapping_add(1));
            } else {
                self.ram
                    .write(COLCRS, self.ram.read(COLCRS).wrapping_add(1));
            }
        }
    }

    fn ensure_text_cursor_defaults(&mut self) {
        if self.ram.read(RMARGIN) == 0 {
            self.ram.write(RMARGIN, 39);
        }
    }

    fn try_service_siov(&mut self, return_pc: u16, cycle: u64) -> Option<u8> {
        if !self.mounted_disks.iter().any(Option::is_some) {
            return None;
        }
        let request = SioRequest::decode(&self.ram);
        if request.device != SIO_DISK_DEVICE {
            return None;
        }

        let sector = matches!(
            request.command,
            SIO_COMMAND_READ_SECTOR | SIO_COMMAND_PUT_SECTOR | SIO_COMMAND_WRITE_SECTOR
        )
        .then_some(request.aux);
        let mut observation = SioObservation {
            cycle,
            return_pc,
            device: request.device,
            unit: request.unit,
            command: request.command,
            direction: request.direction,
            sector,
            buffer: request.buffer,
            length: request.length,
            handled: true,
            status: SIO_STATUS_SUCCESS,
            bytes_transferred: 0,
            detail: String::new(),
        };

        let Some(disk) = self.mounted_disk(request.unit) else {
            observation.status = SIO_STATUS_DEVICE_TIMEOUT;
            observation.detail = "disk unit is not mounted".to_string();
            return Some(self.finish_sio_observation(observation));
        };

        match request.command {
            SIO_COMMAND_STATUS => {
                if request.direction & 0xC0 != SIO_DIRECTION_READ || request.length != 4 {
                    observation.status = SIO_STATUS_DEVICE_ERROR;
                    observation.detail = "status request requires a four-byte read".to_string();
                } else {
                    let status = disk_status_bytes(disk);
                    for (offset, byte) in status.into_iter().enumerate() {
                        self.ram
                            .write(request.buffer.wrapping_add(offset as u16), byte);
                    }
                    observation.bytes_transferred = 4;
                    observation.detail = "read disk status".to_string();
                }
            }
            SIO_COMMAND_READ_SECTOR => {
                let data = disk.image.read_sector(request.aux).map(<[u8]>::to_vec);
                match data {
                    Err(detail) => {
                        observation.status = SIO_STATUS_DEVICE_NAK;
                        observation.detail = detail;
                    }
                    Ok(data)
                        if request.direction & 0xC0 != SIO_DIRECTION_READ
                            || usize::from(request.length) != data.len() =>
                    {
                        observation.status = SIO_STATUS_DEVICE_ERROR;
                        observation.detail = format!(
                            "sector {} requires a {}-byte read, got direction ${:02X} and length {}",
                            request.aux,
                            data.len(),
                            request.direction,
                            request.length
                        );
                    }
                    Ok(data) => {
                        for (offset, byte) in data.iter().copied().enumerate() {
                            self.ram
                                .write(request.buffer.wrapping_add(offset as u16), byte);
                        }
                        observation.bytes_transferred = data.len() as u16;
                        observation.detail = format!("read sector {}", request.aux);
                    }
                }
            }
            SIO_COMMAND_PUT_SECTOR | SIO_COMMAND_WRITE_SECTOR => {
                let write_policy = disk.write_policy;
                match disk.image.sector_len(request.aux) {
                    Err(detail) => {
                        observation.status = SIO_STATUS_DEVICE_NAK;
                        observation.detail = detail;
                    }
                    Ok(_) if write_policy == DiskWritePolicy::ReadOnly => {
                        observation.status = SIO_STATUS_DEVICE_ERROR;
                        observation.detail = "disk is mounted read-only".to_string();
                    }
                    Ok(expected)
                        if request.direction & 0xC0 != SIO_DIRECTION_WRITE
                            || usize::from(request.length) != expected =>
                    {
                        observation.status = SIO_STATUS_DEVICE_ERROR;
                        observation.detail = format!(
                            "sector {} requires a {expected}-byte write, got direction ${:02X} and length {}",
                            request.aux, request.direction, request.length
                        );
                    }
                    Ok(expected) => {
                        let data = (0..request.length)
                            .map(|offset| self.ram.read(request.buffer.wrapping_add(offset)))
                            .collect::<Vec<_>>();
                        self.mounted_disk_mut(request.unit)
                            .expect("disk was resolved before write")
                            .image
                            .write_sector(request.aux, &data)
                            .expect("sector and length were validated before write");
                        observation.bytes_transferred = expected as u16;
                        observation.detail = format!("wrote sector {}", request.aux);
                    }
                }
            }
            SIO_COMMAND_FORMAT | SIO_COMMAND_FORMAT_ENHANCED => {
                let write_policy = disk.write_policy;
                let expected = disk.image.sector_size();
                let sector_count = disk.image.sector_count();
                if write_policy == DiskWritePolicy::ReadOnly {
                    observation.status = SIO_STATUS_DEVICE_ERROR;
                    observation.detail = "disk is mounted read-only".to_string();
                } else if request.direction & 0xC0 != SIO_DIRECTION_READ
                    || usize::from(request.length) != expected
                {
                    observation.status = SIO_STATUS_DEVICE_ERROR;
                    observation.detail = format!(
                        "format requires a {expected}-byte result read, got direction ${:02X} and length {}",
                        request.direction, request.length
                    );
                } else {
                    let format_result = self
                        .mounted_disk_mut(request.unit)
                        .expect("disk was resolved before format")
                        .image
                        .format_sectors(0);
                    match format_result {
                        Err(detail) => {
                            observation.status = SIO_STATUS_DEVICE_ERROR;
                            observation.detail = detail;
                        }
                        Ok(()) => {
                            for offset in 0..request.length {
                                self.ram.write(request.buffer.wrapping_add(offset), 0xFF);
                            }
                            observation.bytes_transferred = request.length;
                            observation.detail = format!(
                                "formatted {} sectors with {}-byte geometry",
                                sector_count, expected
                            );
                        }
                    }
                }
            }
            _ => {
                observation.status = SIO_STATUS_DEVICE_NAK;
                observation.detail = format!("unsupported disk command ${:02X}", request.command);
            }
        }

        Some(self.finish_sio_observation(observation))
    }

    fn finish_sio_observation(&mut self, observation: SioObservation) -> u8 {
        self.ram.write(DSTATS, observation.status);
        self.sio_summary.calls += 1;
        if observation.status == SIO_STATUS_SUCCESS {
            self.sio_summary.successful += 1;
        } else {
            self.sio_summary.errors += 1;
        }
        match observation.command {
            SIO_COMMAND_STATUS => self.sio_summary.statuses += 1,
            SIO_COMMAND_READ_SECTOR => self.sio_summary.reads += 1,
            SIO_COMMAND_PUT_SECTOR | SIO_COMMAND_WRITE_SECTOR => {
                self.sio_summary.writes += 1;
            }
            SIO_COMMAND_FORMAT | SIO_COMMAND_FORMAT_ENHANCED => {
                self.sio_summary.formats += 1;
            }
            _ => {}
        }
        self.sio_summary.bytes_transferred += u64::from(observation.bytes_transferred);
        if self.trace_sio {
            eprintln!(
                "SIO: cyc={} dev=${:02X} unit={} cmd=${:02X} dir=${:02X} sector={} buf=${:04X} len={} status=${:02X} bytes={} {}",
                observation.cycle,
                observation.device,
                observation.unit,
                observation.command,
                observation.direction,
                observation
                    .sector
                    .map(|sector| sector.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                observation.buffer,
                observation.length,
                observation.status,
                observation.bytes_transferred,
                observation.detail
            );
        }
        let status = observation.status;
        if self.sio_observations.len() == SIO_OBSERVATION_LIMIT {
            self.sio_observations.pop_front();
        }
        self.sio_observations.push_back(observation);
        status
    }

    fn start_cio_observation(
        &self,
        x: u8,
        command: u8,
        return_pc: u16,
        cycle: u64,
    ) -> CioObservation {
        let buffer = self.ram.read_word(IOCB_BUFFER_BASE.wrapping_add(x as u16));
        let length = self.ram.read_word(IOCB_LENGTH_BASE.wrapping_add(x as u16));
        let aux1 = self.ram.read(IOCB_AUX1_BASE.wrapping_add(x as u16));
        let aux2 = self.ram.read(IOCB_AUX2_BASE.wrapping_add(x as u16));
        let previous_cycle = self
            .cio_observations
            .back()
            .map(|observation| observation.cycle);
        CioObservation {
            cycle,
            delta_cycles: previous_cycle.map(|previous| cycle.saturating_sub(previous)),
            x,
            channel: cio_channel_index(x).map(|channel| channel as u8),
            command,
            return_pc,
            aux1,
            aux2,
            buffer,
            length,
            device_before: self
                .cio_channel_device(x)
                .map(|device| self.describe_cio_device(device)),
            handled: false,
            detail: String::new(),
            result_a: None,
            result_y: None,
            bytes_read: None,
            bytes_written: None,
            preview: None,
        }
    }

    fn finish_cio_observation(&mut self, observation: CioObservation) {
        self.cio_summary.calls = self.cio_summary.calls.saturating_add(1);
        if observation.handled {
            self.cio_summary.handled = self.cio_summary.handled.saturating_add(1);
        } else {
            self.cio_summary.passthrough = self.cio_summary.passthrough.saturating_add(1);
        }
        match observation.command {
            CIO_COMMAND_OPEN => self.cio_summary.opens = self.cio_summary.opens.saturating_add(1),
            CIO_COMMAND_CLOSE => {
                self.cio_summary.closes = self.cio_summary.closes.saturating_add(1)
            }
            CIO_COMMAND_STATUS => {
                self.cio_summary.statuses = self.cio_summary.statuses.saturating_add(1)
            }
            CIO_COMMAND_GETREC | CIO_COMMAND_GETCHR => {
                self.cio_summary.reads = self.cio_summary.reads.saturating_add(1);
                self.cio_summary.bytes_read = self
                    .cio_summary
                    .bytes_read
                    .saturating_add(observation.bytes_read.unwrap_or(0) as u64);
                if matches!(observation.result_y, Some(0x88)) {
                    self.cio_summary.eof = self.cio_summary.eof.saturating_add(1);
                }
            }
            CIO_COMMAND_PUTREC | CIO_COMMAND_PUTCHR => {
                self.cio_summary.writes = self.cio_summary.writes.saturating_add(1);
                self.cio_summary.bytes_written = self
                    .cio_summary
                    .bytes_written
                    .saturating_add(observation.bytes_written.unwrap_or(0) as u64);
            }
            _ => {}
        }
        if self.cio_observations.len() == CIO_OBSERVATION_LIMIT {
            self.cio_observations.pop_front();
        }
        self.cio_observations.push_back(observation);
    }

    fn trace_cio_call(&self, observation: &CioObservation) {
        if !self.trace_cio {
            return;
        }
        eprintln!(
            "CIO x=${:02X} ch={} cmd=${:02X} ret=${:04X} aux=${:02X}/${:02X} buf=${:04X} len={} dev={}",
            observation.x,
            observation.channel.unwrap_or(0xFF),
            observation.command,
            observation.return_pc,
            observation.aux1,
            observation.aux2,
            observation.buffer,
            observation.length,
            observation.device_before.as_deref().unwrap_or("-")
        );
    }

    fn trace_cio(&self, args: std::fmt::Arguments<'_>) {
        if self.trace_cio {
            eprintln!("{args}");
        }
    }

    fn describe_cio_device(&self, device: CioHarnessDevice) -> String {
        match device {
            CioHarnessDevice::QueuedInput => "Q:".to_string(),
            CioHarnessDevice::Editor => "E:".to_string(),
            CioHarnessDevice::Screen => "S:".to_string(),
            CioHarnessDevice::Host { file_index, offset } => self
                .host_files
                .get(file_index)
                .map(|file| format!("{}@{offset}", file.name))
                .unwrap_or_else(|| format!("#{file_index}@{offset}")),
        }
    }

    fn read_self_test(&self, address: u16) -> Option<u8> {
        if !self.self_test_rom_enabled {
            return None;
        }
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
    Editor,
    Screen,
    Host { file_index: usize, offset: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CioReadResult {
    accumulator: u8,
    status: u8,
    bytes_read: usize,
    detail: String,
    preview: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SioRequest {
    device: u8,
    unit: u8,
    command: u8,
    direction: u8,
    buffer: u16,
    length: u16,
    aux: u16,
}

impl SioRequest {
    fn decode(memory: &Memory) -> Self {
        Self {
            device: memory.read(DDEVIC),
            unit: memory.read(DUNIT),
            command: memory.read(DCOMND),
            direction: memory.read(DSTATS),
            buffer: memory.read_word(DBUFLO),
            length: memory.read_word(DBYTLO),
            aux: memory.read_word(DAUX1),
        }
    }
}

fn disk_unit_index(unit: u8) -> Result<usize, String> {
    if !(1..=8).contains(&unit) {
        return Err(format!("disk unit must be in 1..=8, got {unit}"));
    }
    Ok(usize::from(unit - 1))
}

fn disk_status_bytes(disk: &MountedDisk) -> [u8; 4] {
    let density = if disk.image.sector_size() == 256 {
        0x20
    } else if disk.image.sector_count() == 1040 {
        0x80
    } else {
        0x00
    };
    [density, 0xFF, 0xE0, 0x00]
}

fn normalize_host_file_name(name: &str) -> String {
    let trimmed = name.trim();
    let without_device = trimmed
        .strip_prefix("H:")
        .or_else(|| trimmed.strip_prefix("h:"))
        .or_else(|| trimmed.strip_prefix("D:"))
        .or_else(|| trimmed.strip_prefix("d:"))
        .unwrap_or(trimmed);
    without_device.trim().to_ascii_uppercase()
}

fn host_source_byte_to_atascii(byte: u8) -> u8 {
    match byte {
        b'\n' => 0x9B,
        _ => byte,
    }
}

fn format_cio_preview(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        match *byte {
            0x9B => output.push_str("\\n"),
            b'\\' => output.push_str("\\\\"),
            b'"' => output.push_str("\\\""),
            0x20..=0x7E => output.push(*byte as char),
            value => output.push_str(&format!("\\x{value:02X}")),
        }
    }
    output
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtectedCodeWrite {
    pub address: u16,
    pub old_value: u8,
    pub new_value: u8,
    pub region: BusRegion,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CioSummary {
    pub calls: u64,
    pub handled: u64,
    pub passthrough: u64,
    pub opens: u64,
    pub closes: u64,
    pub statuses: u64,
    pub reads: u64,
    pub writes: u64,
    pub eof: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CioObservation {
    pub cycle: u64,
    pub delta_cycles: Option<u64>,
    pub x: u8,
    pub channel: Option<u8>,
    pub command: u8,
    pub return_pc: u16,
    pub aux1: u8,
    pub aux2: u8,
    pub buffer: u16,
    pub length: u16,
    pub device_before: Option<String>,
    pub handled: bool,
    pub detail: String,
    pub result_a: Option<u8>,
    pub result_y: Option<u8>,
    pub bytes_read: Option<u16>,
    pub bytes_written: Option<u16>,
    pub preview: Option<String>,
}

/// Cumulative counters for disk requests intercepted at the OS `SIOV` entry.
///
/// Unlike the bounded recent-observation queue, these counters cover the
/// complete lifetime of the bus.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SioSummary {
    pub calls: u64,
    pub successful: u64,
    pub errors: u64,
    pub statuses: u64,
    pub reads: u64,
    pub writes: u64,
    pub formats: u64,
    pub bytes_transferred: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SioObservation {
    pub cycle: u64,
    pub return_pc: u16,
    pub device: u8,
    pub unit: u8,
    pub command: u8,
    pub direction: u8,
    pub sector: Option<u16>,
    pub buffer: u16,
    pub length: u16,
    pub handled: bool,
    pub status: u8,
    pub bytes_transferred: u16,
    pub detail: String,
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
    portb_data: u8,
    portb_ddr: u8,
    portb_control: u8,
    console_switches: u8,
    speaker_write_count: u64,
    last_speaker_write: Option<u8>,
}

impl Default for IoRegion {
    fn default() -> Self {
        Self {
            range: AddressRange::with_size(IO_BASE, IO_SIZE).expect("valid I/O range"),
            bytes: vec![0xFF; IO_SIZE],
            portb_data: 0xFF,
            portb_ddr: 0xFF,
            portb_control: 0xFF,
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
        if address == PORTB {
            if self.portb_control & PIA_DDR_ACCESS_DISABLE != 0 {
                return Some(self.portb_effective());
            }
            return Some(self.portb_ddr);
        }
        if address == PBCTL_PORTB_CONTROL {
            return Some(self.portb_control);
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
        if address == PORTB {
            if self.portb_control & PIA_DDR_ACCESS_DISABLE != 0 {
                self.portb_data = value;
            } else {
                self.portb_ddr = value;
            }
            self.bytes[(address - self.range.start) as usize] = value;
            return true;
        }
        if address == PBCTL_PORTB_CONTROL {
            self.portb_control = value;
            self.bytes[(address - self.range.start) as usize] = value;
            return true;
        }
        self.bytes[(address - self.range.start) as usize] = value;
        true
    }

    pub fn portb(&self) -> u8 {
        self.portb_effective()
    }

    fn disable_self_test_rom(&mut self) {
        self.portb_data |= PORTB_SELF_TEST_DISABLE;
        self.portb_ddr |= PORTB_SELF_TEST_DISABLE;
        self.bytes[(PORTB - self.range.start) as usize] = self.portb_effective();
    }

    fn portb_effective(&self) -> u8 {
        self.portb_data | !self.portb_ddr
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

#[cfg(test)]
mod tests {
    use super::*;

    const TN_STANDALONE_OBJECT: &[u8] = include_bytes!("../tests/fixtures/tn-standalone.com");
    const TN_FILE_COUNT: u16 = 0x2C61;
    const TN_NEST_LEVEL: u16 = 0x2C62;
    const TN_ACTIVE_PANEL: u16 = 0x2C71;
    const TN_SWAP_RIGHT_KEY: u8 = 0x07;
    const TN_SWAP_LEFT_KEY: u8 = 0x06;
    const TN_VIEW_KEY: u8 = 0x50;
    const TN_COPY_KEY: u8 = 0x52;
    const TN_KEYBOARD_WAIT_PC: u16 = 0xFE77;
    const TN_UI_STEP_LIMIT: usize = 15_000_000;

    struct TnHarness {
        vm: CompilerVm,
    }

    impl TnHarness {
        fn boot_bundled(disks: &[(u8, DiskWritePolicy)]) -> Self {
            let mut vm = CompilerVm::default();
            for (unit, policy) in disks {
                vm.mount_bundled_mydos(*unit, *policy).unwrap();
            }
            vm.prepare_execution_profile(ExecutionProfile::DiskBoot)
                .unwrap();
            vm.reset_cpu();
            let mut harness = Self { vm };
            harness.wait_until("MyDOS command prompt", 400_000, |vm| {
                vm.bus().dos_boot_is_ready() && vm.cpu().registers().pc == 0xEA2D
            });
            harness
        }

        fn launch(&mut self) {
            self.vm.bus_mut().ram_mut().write(0x070A, 0xFF);
            let report = self.vm.load_atari_object(TN_STANDALONE_OBJECT).unwrap();
            self.vm.set_pc(report.run_address.unwrap());
            self.wait_for_keyboard("TN main screen");
            self.assert_screen_contains("Toms Navigator");
        }

        fn wait_until<F>(&mut self, context: &str, max_steps: usize, mut predicate: F)
        where
            F: FnMut(&CompilerVm) -> bool,
        {
            for _ in 0..max_steps {
                if predicate(&self.vm) {
                    return;
                }
                self.vm.step_cpu().unwrap();
            }
            if predicate(&self.vm) {
                return;
            }
            panic!(
                "timed out waiting for {context}; PC=${:04X}\n{}",
                self.vm.cpu().registers().pc,
                self.screen_text()
            );
        }

        fn wait_for_keyboard(&mut self, context: &str) {
            self.wait_until(context, TN_UI_STEP_LIMIT, |vm| {
                // The bundled AltirraOS waits for K: input at $FE77. Requiring
                // both the hardware latch and VM queue to be empty prevents a
                // multi-key command from looking complete between characters.
                vm.cpu().registers().pc == TN_KEYBOARD_WAIT_PC
                    && vm.bus().ram().read(CH_KEY_CODE) == 0xFF
                    && vm.bus().pending_key_codes.is_empty()
            });
        }

        fn send_key(&mut self, key_code: u8, context: &str) {
            self.vm.bus_mut().queue_key_code(key_code);
            self.wait_for_keyboard(context);
        }

        fn send_text(&mut self, characters: &[u8], context: &str) {
            queue_keyboard_text(&mut self.vm, characters);
            self.wait_for_keyboard(context);
        }

        fn switch_panel(&mut self, panel: u8) {
            assert!(panel <= 1);
            if self.vm.bus().ram().read(TN_ACTIVE_PANEL) != panel {
                let key = if panel == 0 {
                    TN_SWAP_LEFT_KEY
                } else {
                    TN_SWAP_RIGHT_KEY
                };
                self.send_key(key, "TN panel switch");
            }
            assert_eq!(self.vm.bus().ram().read(TN_ACTIVE_PANEL), panel);
        }

        fn select_drive(&mut self, unit: u8) {
            assert!((1..=8).contains(&unit));
            self.send_text(&[b'0' + unit], "TN drive selection");
            self.assert_screen_contains(&format!("D{unit}>"));
        }

        fn assert_screen_contains(&self, needle: &str) {
            assert!(
                self.vm
                    .bus()
                    .text_screen_snapshot(40, 24)
                    .lines
                    .iter()
                    .any(|line| line.contains(needle)),
                "screen does not contain `{needle}`; PC=${:04X}\n{}",
                self.vm.cpu().registers().pc,
                self.screen_text()
            );
        }

        fn assert_screen_lacks(&self, needle: &str) {
            assert!(
                self.vm
                    .bus()
                    .text_screen_snapshot(40, 24)
                    .lines
                    .iter()
                    .all(|line| !line.contains(needle)),
                "screen still contains `{needle}`; PC=${:04X}\n{}",
                self.vm.cpu().registers().pc,
                self.screen_text()
            );
        }

        fn screen_text(&self) -> String {
            self.vm.bus().text_screen_snapshot(40, 24).lines.join("\n")
        }
    }

    fn test_atr_bytes(sector_size: usize, sectors: usize) -> Vec<u8> {
        let payload = if sector_size == 128 {
            sectors * 128
        } else {
            sectors.min(3) * 128 + sectors.saturating_sub(3) * 256
        };
        let paragraphs = payload / 16;
        let mut bytes = vec![0; 16 + payload];
        bytes[0..2].copy_from_slice(&0x0296u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&(paragraphs as u16).to_le_bytes());
        bytes[4..6].copy_from_slice(&(sector_size as u16).to_le_bytes());
        bytes[6] = (paragraphs >> 16) as u8;
        bytes
    }

    fn prepare_siov_call(
        bus: &mut Bus,
        cpu: &mut Cpu,
        unit: u8,
        command: u8,
        direction: u8,
        buffer: u16,
        length: u16,
        aux: u16,
    ) {
        bus.ram_mut().write(DDEVIC, SIO_DISK_DEVICE);
        bus.ram_mut().write(DUNIT, unit);
        bus.ram_mut().write(DCOMND, command);
        bus.ram_mut().write(DSTATS, direction);
        bus.ram_mut().write_word(DBUFLO, buffer);
        bus.ram_mut().write_word(DBYTLO, length);
        bus.ram_mut().write_word(DAUX1, aux);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = SIOV;
        cpu.registers.sp = 0xFB;
    }

    fn run_native_ciov(vm: &mut CompilerVm, x: u8) -> Result<CpuRegisters, String> {
        const TRAMPOLINE: u16 = 0x4000;
        vm.bus_mut().ram_mut().map(
            TRAMPOLINE,
            &[
                0x20,
                CIOV as u8,
                (CIOV >> 8) as u8, // JSR CIOV
                0x4C,
                0x03,
                0x40, // JMP $4003
            ],
        )?;
        vm.cpu.registers.pc = TRAMPOLINE;
        vm.cpu.registers.x = x;
        for _ in 0..2_000_000 {
            vm.step_cpu().map_err(|err| format!("{err:?}"))?;
            if vm.cpu().registers().pc == TRAMPOLINE + 3 {
                return Ok(vm.cpu().registers());
            }
        }
        Err("native CIO call did not return to its trampoline".to_string())
    }

    fn configure_iocb(vm: &mut CompilerVm, x: u8, command: u8, buffer: u16, length: u16, aux1: u8) {
        let ram = vm.bus_mut().ram_mut();
        ram.write(IOCB_COMMAND_BASE.wrapping_add(x as u16), command);
        ram.write_word(IOCB_BUFFER_BASE.wrapping_add(x as u16), buffer);
        ram.write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), length);
        if command == CIO_COMMAND_OPEN {
            ram.write(IOCB_AUX1_BASE.wrapping_add(x as u16), aux1);
            ram.write(IOCB_AUX2_BASE.wrapping_add(x as u16), 0);
        }
    }

    fn boot_bundled_mydos_to_prompt(policy: DiskWritePolicy) -> CompilerVm {
        let mut vm = CompilerVm::default();
        vm.mount_bundled_mydos(1, policy).unwrap();
        vm.prepare_execution_profile(ExecutionProfile::DiskBoot)
            .unwrap();
        vm.reset_cpu();
        for _ in 0..400_000 {
            if vm.bus().dos_boot_is_ready() && vm.cpu().registers().pc == 0xEA2D {
                return vm;
            }
            vm.step_cpu().unwrap();
        }
        panic!(
            "bundled MyDOS did not reach its command prompt; PC=${:04X}",
            vm.cpu().registers().pc
        );
    }

    fn native_cio_filename_command(
        vm: &mut CompilerVm,
        command: u8,
        name: &[u8],
        aux1: u8,
        aux2: u8,
    ) -> u8 {
        const X: u8 = 0x10;
        const FILENAME: u16 = 0x4200;

        vm.bus_mut().ram_mut().map(FILENAME, name).unwrap();
        let ram = vm.bus_mut().ram_mut();
        ram.write(IOCB_DEVICE_BASE.wrapping_add(u16::from(X)), 0xFF);
        ram.write(IOCB_COMMAND_BASE.wrapping_add(u16::from(X)), command);
        ram.write_word(IOCB_BUFFER_BASE.wrapping_add(u16::from(X)), FILENAME);
        ram.write_word(IOCB_LENGTH_BASE.wrapping_add(u16::from(X)), 0);
        ram.write(IOCB_AUX1_BASE.wrapping_add(u16::from(X)), aux1);
        ram.write(IOCB_AUX2_BASE.wrapping_add(u16::from(X)), aux2);
        let status = run_native_ciov(vm, X).unwrap().y;
        if command != CIO_COMMAND_OPEN {
            configure_iocb(vm, X, CIO_COMMAND_CLOSE, 0, 0, 0);
            let _ = run_native_ciov(vm, X).unwrap();
        }
        status
    }

    fn native_file_open_status(vm: &mut CompilerVm, name: &[u8]) -> u8 {
        const X: u8 = 0x10;
        let status = native_cio_filename_command(vm, CIO_COMMAND_OPEN, name, 4, 0);
        configure_iocb(vm, X, CIO_COMMAND_CLOSE, 0, 0, 0);
        let _ = run_native_ciov(vm, X).unwrap();
        status
    }

    fn native_write_file(vm: &mut CompilerVm, name: &[u8], bytes: &[u8]) {
        const X: u8 = 0x10;
        const FILENAME: u16 = 0x4200;
        const DATA: u16 = 0x6000;
        const CHUNK_SIZE: usize = 0x2000;

        vm.bus_mut().ram_mut().map(FILENAME, name).unwrap();
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_DEVICE_BASE.wrapping_add(u16::from(X)), 0xFF);
        configure_iocb(vm, X, CIO_COMMAND_OPEN, FILENAME, 0, 8);
        let open_status = run_native_ciov(vm, X).unwrap().y;
        assert_eq!(
            open_status,
            1,
            "failed to create {}",
            format_cio_preview(name)
        );
        for chunk in bytes.chunks(CHUNK_SIZE) {
            vm.bus_mut().ram_mut().map(DATA, chunk).unwrap();
            configure_iocb(vm, X, CIO_COMMAND_PUTCHR, DATA, chunk.len() as u16, 0);
            assert_eq!(run_native_ciov(vm, X).unwrap().y, 1);
        }
        configure_iocb(vm, X, CIO_COMMAND_CLOSE, 0, 0, 0);
        assert_eq!(run_native_ciov(vm, X).unwrap().y, 1);
    }

    fn native_read_file(vm: &mut CompilerVm, name: &[u8], length: usize) -> Vec<u8> {
        const X: u8 = 0x10;
        const FILENAME: u16 = 0x4200;
        const DATA: u16 = 0x6000;
        const CHUNK_SIZE: usize = 0x2000;

        vm.bus_mut().ram_mut().map(FILENAME, name).unwrap();
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_DEVICE_BASE.wrapping_add(u16::from(X)), 0xFF);
        configure_iocb(vm, X, CIO_COMMAND_OPEN, FILENAME, 0, 4);
        assert_eq!(run_native_ciov(vm, X).unwrap().y, 1);
        let mut bytes = Vec::with_capacity(length);
        while bytes.len() < length {
            let chunk_length = CHUNK_SIZE.min(length - bytes.len());
            configure_iocb(vm, X, CIO_COMMAND_GETCHR, DATA, chunk_length as u16, 0);
            let read_status = run_native_ciov(vm, X).unwrap().y;
            assert!(
                read_status < 0x80,
                "native read failed with ${read_status:02X}"
            );
            bytes.extend(
                (0..chunk_length as u16)
                    .map(|offset| vm.bus().ram().read(DATA.wrapping_add(offset))),
            );
        }
        configure_iocb(vm, X, CIO_COMMAND_CLOSE, 0, 0, 0);
        assert_eq!(run_native_ciov(vm, X).unwrap().y, 1);
        bytes
    }

    fn queue_keyboard_text(vm: &mut CompilerVm, characters: &[u8]) {
        let table = vm.bus().ram().read_word(0x0079);
        let key_codes = characters
            .iter()
            .map(|character| {
                (0u8..=0x7F)
                    .find(|key_code| {
                        vm.bus_mut().read(table.wrapping_add(u16::from(*key_code))) == *character
                    })
                    .unwrap_or_else(|| panic!("no keyboard code for ${character:02X}"))
            })
            .collect::<Vec<_>>();
        for key_code in key_codes {
            vm.bus_mut().queue_key_code(key_code);
        }
    }

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
    fn mounts_numbered_atr_images_without_host_paths() {
        let mut vm = CompilerVm::default();
        let bytes = test_atr_bytes(128, 720);

        vm.mount_atr_bytes(1, bytes.clone(), DiskWritePolicy::ReadOnly)
            .unwrap();

        assert_eq!(vm.mounted_atr_bytes(1), Some(bytes));
        assert!(!vm.disk_is_dirty(1));
        assert!(
            vm.mount_atr_bytes(0, test_atr_bytes(128, 1), DiskWritePolicy::ReadOnly)
                .is_err()
        );
        assert_eq!(vm.bus_mut().unmount_disk(1).unwrap().unwrap().unit, 1);
        assert!(vm.mounted_atr_bytes(1).is_none());
    }

    #[test]
    fn cpu_services_siov_sector_reads_and_matches_os_return_state() {
        let mut bytes = test_atr_bytes(256, 5);
        let sector_four = 16 + 3 * 128;
        for (index, byte) in bytes[sector_four..sector_four + 256].iter_mut().enumerate() {
            *byte = index as u8;
        }
        let mut bus = Bus::default();
        bus.mount_atr_bytes(1, bytes, DiskWritePolicy::ReadOnly)
            .unwrap();
        let mut cpu = Cpu::default();
        prepare_siov_call(
            &mut bus,
            &mut cpu,
            1,
            SIO_COMMAND_READ_SECTOR,
            SIO_DIRECTION_READ,
            0x4000,
            256,
            4,
        );

        let step = cpu.step(&mut bus).unwrap();

        assert_eq!(step.pc, SIOV);
        assert_eq!(step.opcode, 0xFF);
        assert_eq!(cpu.registers.pc, 0x2000);
        assert_eq!(cpu.registers.sp, 0xFD);
        assert_eq!(cpu.registers.a, 0);
        assert_eq!(cpu.registers.y, SIO_STATUS_SUCCESS);
        assert_ne!(cpu.registers.status & StatusFlags::CARRY.bits(), 0);
        assert_eq!(cpu.registers.status & StatusFlags::NEGATIVE.bits(), 0);
        assert_eq!(bus.ram().read(DSTATS), SIO_STATUS_SUCCESS);
        assert_eq!(bus.ram().read(0x4000), 0);
        assert_eq!(bus.ram().read(0x40FF), 0xFF);
        assert_eq!(bus.sio_observations().len(), 1);
        assert_eq!(bus.sio_observations()[0].bytes_transferred, 256);
        assert_eq!(bus.sio_observations()[0].sector, Some(4));
    }

    #[test]
    fn cpu_services_siov_status_with_atr_density() {
        let mut bus = Bus::default();
        bus.mount_atr_bytes(1, test_atr_bytes(256, 720), DiskWritePolicy::ReadOnly)
            .unwrap();
        let mut cpu = Cpu::default();
        prepare_siov_call(
            &mut bus,
            &mut cpu,
            1,
            SIO_COMMAND_STATUS,
            SIO_DIRECTION_READ,
            0x4000,
            4,
            0,
        );

        cpu.step(&mut bus).unwrap();

        assert_eq!(
            (0..4)
                .map(|offset| bus.ram().read(0x4000 + offset))
                .collect::<Vec<_>>(),
            vec![0x20, 0xFF, 0xE0, 0x00]
        );
        assert_eq!(bus.sio_observations()[0].bytes_transferred, 4);
    }

    #[test]
    fn cpu_writes_copy_on_write_sectors_and_reads_them_back() {
        let original = test_atr_bytes(128, 4);
        let mut bus = Bus::default();
        bus.mount_atr_bytes(1, original.clone(), DiskWritePolicy::CopyOnWrite)
            .unwrap();
        for offset in 0..128u16 {
            bus.ram_mut().write(0x4000 + offset, offset as u8 ^ 0xA5);
        }
        let mut cpu = Cpu::default();
        prepare_siov_call(
            &mut bus,
            &mut cpu,
            1,
            SIO_COMMAND_WRITE_SECTOR,
            SIO_DIRECTION_WRITE,
            0x4000,
            128,
            2,
        );

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.y, SIO_STATUS_SUCCESS);
        assert_eq!(bus.sio_observations()[0].bytes_transferred, 128);
        assert!(bus.disk_is_dirty(1));
        assert_eq!(bus.dirty_disk_sectors(1), Some(vec![2]));
        assert_eq!(bus.original_atr_bytes(1), Some(original));

        for offset in 0..128u16 {
            bus.ram_mut().write(0x4100 + offset, 0);
        }
        prepare_siov_call(
            &mut bus,
            &mut cpu,
            1,
            SIO_COMMAND_READ_SECTOR,
            SIO_DIRECTION_READ,
            0x4100,
            128,
            2,
        );
        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.y, SIO_STATUS_SUCCESS);
        for offset in 0..128u16 {
            assert_eq!(bus.ram().read(0x4100 + offset), offset as u8 ^ 0xA5);
        }
    }

    #[test]
    fn cpu_formats_copy_on_write_disk_and_returns_no_bad_sectors() {
        let mut original = test_atr_bytes(256, 5);
        original[16..].fill(0xA5);
        let mut bus = Bus::default();
        bus.mount_atr_bytes(1, original, DiskWritePolicy::CopyOnWrite)
            .unwrap();
        let mut cpu = Cpu::default();
        prepare_siov_call(
            &mut bus,
            &mut cpu,
            1,
            SIO_COMMAND_FORMAT,
            SIO_DIRECTION_READ,
            0x4000,
            256,
            0,
        );

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.y, SIO_STATUS_SUCCESS);
        assert!((0..256u16).all(|offset| bus.ram().read(0x4000 + offset) == 0xFF));
        assert!(
            bus.mounted_atr_bytes(1).unwrap()[16..]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(bus.dirty_disk_sectors(1), Some(vec![1, 2, 3, 4, 5]));
        assert_eq!(bus.sio_observations()[0].bytes_transferred, 256);
        assert!(
            bus.sio_observations()[0]
                .detail
                .contains("formatted 5 sectors")
        );
        assert_eq!(
            bus.sio_summary(),
            &SioSummary {
                calls: 1,
                successful: 1,
                formats: 1,
                bytes_transferred: 256,
                ..SioSummary::default()
            }
        );
    }

    #[test]
    fn cpu_reports_deterministic_siov_disk_errors() {
        let cases = [
            (
                1,
                SIO_COMMAND_READ_SECTOR,
                SIO_DIRECTION_READ,
                128,
                1,
                SIO_STATUS_DEVICE_TIMEOUT,
            ),
            (
                2,
                SIO_COMMAND_READ_SECTOR,
                SIO_DIRECTION_READ,
                128,
                0,
                SIO_STATUS_DEVICE_NAK,
            ),
            (
                2,
                SIO_COMMAND_READ_SECTOR,
                SIO_DIRECTION_READ,
                127,
                1,
                SIO_STATUS_DEVICE_ERROR,
            ),
            (
                2,
                SIO_COMMAND_WRITE_SECTOR,
                SIO_DIRECTION_WRITE,
                128,
                1,
                SIO_STATUS_DEVICE_ERROR,
            ),
            (
                2,
                SIO_COMMAND_FORMAT,
                SIO_DIRECTION_READ,
                128,
                0,
                SIO_STATUS_DEVICE_ERROR,
            ),
            (2, 0x99, 0, 0, 0, SIO_STATUS_DEVICE_NAK),
        ];

        for (unit, command, direction, length, aux, expected_status) in cases {
            let mut bus = Bus::default();
            bus.mount_atr_bytes(2, test_atr_bytes(128, 4), DiskWritePolicy::ReadOnly)
                .unwrap();
            let mut cpu = Cpu::default();
            prepare_siov_call(
                &mut bus, &mut cpu, unit, command, direction, 0x4000, length, aux,
            );

            cpu.step(&mut bus).unwrap();

            assert_eq!(cpu.registers.y, expected_status);
            assert_eq!(bus.ram().read(DSTATS), expected_status);
            assert_ne!(cpu.registers.status & StatusFlags::NEGATIVE.bits(), 0);
            assert_eq!(bus.sio_observations().len(), 1);
            assert_eq!(bus.sio_observations()[0].status, expected_status);
        }
    }

    #[test]
    fn cpu_leaves_siov_to_the_os_when_disk_service_is_inactive() {
        let mut bus = Bus::default();
        let mut cpu = Cpu::default();
        prepare_siov_call(
            &mut bus,
            &mut cpu,
            1,
            SIO_COMMAND_READ_SECTOR,
            SIO_DIRECTION_READ,
            0x4000,
            128,
            1,
        );

        assert!(!cpu.try_emulate_siov(&mut bus));
        assert_eq!(cpu.registers.pc, SIOV);
        assert!(bus.sio_observations().is_empty());
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
    fn constructs_vm_inputs_from_caller_owned_bytes() {
        let mut vm = CompilerVm::default();
        vm.load_image_bytes(
            ImageKind::Ram,
            "embedded:test-program",
            0x2000,
            vec![0xA9, 0x42],
        )
        .unwrap();
        vm.set_source_bytes(b"BYTE value".to_vec());
        vm.add_host_file_bytes("D:LIB.ACT", b"BYTE helper".to_vec());
        vm.add_host_output("D:OUT.COM");
        vm.set_trace_cio(true);

        assert_eq!(vm.memory().read(0x2000), 0xA9);
        assert_eq!(vm.source(), Some(b"BYTE value".as_slice()));
        assert_eq!(
            vm.host_file_bytes("lib.act"),
            Some(b"BYTE helper".as_slice())
        );
        assert_eq!(vm.host_file_bytes("out.com"), Some([].as_slice()));
        assert_eq!(vm.images().len(), 1);
        assert_eq!(vm.images()[0].path, PathBuf::from("embedded:test-program"));

        vm.clear_source();
        assert_eq!(vm.source(), None);
    }

    #[test]
    fn byte_image_diagnostics_retain_the_caller_label() {
        let mut vm = CompilerVm::default();
        let error = vm
            .load_image_bytes(ImageKind::Ram, "generated-object", 0xFFFF, vec![0x11, 0x22])
            .unwrap_err();

        assert!(error.contains("generated-object"));
        assert!(error.contains("exceeds 64K"));
    }

    #[test]
    fn run_configuration_defaults_to_bundled_action_environment() {
        let config = VmConfig::default();
        config.validate_for_execution().unwrap();
        let vm = config
            .load_for_profile(ExecutionProfile::OriginalCompiler)
            .unwrap();
        assert_eq!(vm.images().len(), 2);
        assert_eq!(
            vm.images()[0].path,
            PathBuf::from(BUNDLED_ACTION_CARTRIDGE_LABEL)
        );
        assert_eq!(vm.images()[1].path, PathBuf::from(BUNDLED_ALTIRRA_OS_LABEL));

        VmConfig::default()
            .validate_for_profile(ExecutionProfile::StandaloneObject)
            .unwrap();
        VmConfig::default()
            .validate_for_profile(ExecutionProfile::SyntheticTest)
            .unwrap();
        assert!(
            VmConfig::default()
                .validate_for_profile(ExecutionProfile::DiskBoot)
                .unwrap_err()
                .contains("drive 1")
        );
    }

    #[test]
    fn configuration_hotpatches_run_after_bundled_images_are_loaded() {
        let config = VmConfig {
            hotpatches: vec![Hotpatch::ActionQueuedInput, Hotpatch::ActionHeadlessGetkey],
            ..VmConfig::default()
        };
        let vm = config
            .load_for_profile(ExecutionProfile::OriginalCompiler)
            .unwrap();
        let cartridge = vm.bus().cartridge().unwrap();

        assert!(
            cartridge
                .payload()
                .windows(9)
                .any(|window| { window == [0x02, b'Q', b':', 0xAD, 0xFC, 0x02, 0x49, 0xFF, 0x60] })
        );
        assert!(cartridge.payload().windows(13).any(|window| {
            window
                == [
                    0xA2, 0x70, 0xA9, 0x07, 0x85, 0x11, 0x20, 0x40, 0xB3, 0x8D, 0xA2, 0x04, 0x60,
                ]
        }));
    }

    #[test]
    fn profile_preparation_uses_bundled_images_only_when_needed() {
        let mut standalone = CompilerVm::default();
        standalone
            .prepare_execution_profile(ExecutionProfile::StandaloneObject)
            .unwrap();
        assert!(standalone.bus().os_rom().is_none());
        assert!(standalone.images().is_empty());

        let mut vm = CompilerVm::default();
        vm.prepare_execution_profile(ExecutionProfile::OriginalCompiler)
            .unwrap();
        let os = vm.bus().os_rom().unwrap();
        assert_eq!(
            os.range(),
            AddressRange::with_size(OS_ROM_BASE, 0x4000).unwrap()
        );
        assert_eq!(vm.images().len(), 2);
        assert_eq!(
            vm.images()[0].path,
            PathBuf::from(BUNDLED_ACTION_CARTRIDGE_LABEL)
        );
        assert_eq!(vm.images()[0].metadata.checksum16, 0x765D);
        assert_eq!(vm.images()[0].metadata.crc32, 0xA1F9_0DFD);
        assert_eq!(vm.images()[0].car_header.unwrap().cartridge_type, 0x0F);
        assert_eq!(vm.images()[1].path, PathBuf::from(BUNDLED_ALTIRRA_OS_LABEL));
        assert_eq!(vm.images()[1].metadata.checksum16, 0x4D75);
        assert_eq!(vm.images()[1].metadata.crc32, 0x5890_AE8E);

        let mut disk_boot = CompilerVm::default();
        disk_boot
            .mount_atr_bytes(1, test_atr_bytes(128, 720), DiskWritePolicy::ReadOnly)
            .unwrap();
        disk_boot
            .prepare_execution_profile(ExecutionProfile::DiskBoot)
            .unwrap();
        assert!(disk_boot.bus().cartridge().is_none());
        assert!(disk_boot.bus().os_rom().is_some());
        assert!(disk_boot.bus().disk_boot_mode());
        assert_eq!(
            disk_boot.bus().cio_fallback_policy(),
            CioFallbackPolicy::NativeOs
        );
        assert_eq!(disk_boot.images().len(), 1);
        assert_eq!(
            disk_boot.images()[0].path,
            PathBuf::from(BUNDLED_ALTIRRA_OS_LABEL)
        );
    }

    #[test]
    fn bundled_mydos_reaches_a_reproducible_native_dos_ready_state() {
        let mut vm = CompilerVm::default();
        vm.mount_bundled_mydos(1, DiskWritePolicy::ReadOnly)
            .unwrap();
        vm.prepare_execution_profile(ExecutionProfile::DiskBoot)
            .unwrap();
        vm.reset_cpu();

        let mut steps = 0u64;
        while steps < 400_000 && !vm.bus().dos_boot_is_ready() {
            vm.step_cpu().unwrap();
            steps += 1;
        }

        assert!(
            vm.bus().dos_boot_is_ready(),
            "MyDOS was not ready after {steps} steps"
        );
        assert!(vm.bus().cartridge().is_none());
        assert_eq!(vm.bus().ram().read_word(DOSVEC_START_VECTOR), 0x1B52);
        assert_eq!(
            vm.bus().ram().read_word(DOSINI_INITIALIZATION_VECTOR),
            0x07E0
        );
        assert_eq!(vm.bus().cio_handler_address(b'D'), Some(0x07D4));
        assert!(vm.bus().sio_observations().iter().all(|observation| {
            matches!(
                observation.command,
                SIO_COMMAND_STATUS | SIO_COMMAND_READ_SECTOR
            ) && matches!(
                observation.status,
                SIO_STATUS_SUCCESS | SIO_STATUS_DEVICE_TIMEOUT
            )
        }));
        assert!(
            vm.bus()
                .sio_observations()
                .iter()
                .any(|observation| observation.command == SIO_COMMAND_READ_SECTOR)
        );
    }

    #[test]
    fn bundled_mydos_lists_its_directory_through_native_cio() {
        let mut vm = CompilerVm::default();
        vm.add_host_file_bytes("DOS.SYS", b"harness shadow".to_vec());
        vm.mount_bundled_mydos(1, DiskWritePolicy::ReadOnly)
            .unwrap();
        vm.prepare_execution_profile(ExecutionProfile::DiskBoot)
            .unwrap();
        vm.reset_cpu();

        for _ in 0..400_000 {
            if vm.bus().dos_boot_is_ready() && vm.cpu().registers().pc == 0xEA2D {
                break;
            }
            vm.step_cpu().unwrap();
        }
        assert!(vm.bus().dos_boot_is_ready());
        assert_eq!(vm.cpu().registers().pc, 0xEA2D);

        let x = 0x10;
        let filename = 0x4200;
        let directory_record = 0x4300;
        vm.bus_mut().ram_mut().map(filename, b"D:*.*\x9B").unwrap();
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_DEVICE_BASE.wrapping_add(x as u16), 0xFF);
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(x as u16), CIO_COMMAND_OPEN);
        vm.bus_mut()
            .ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(x as u16), filename);
        vm.bus_mut()
            .ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), 0);
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_AUX1_BASE.wrapping_add(x as u16), 6);
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_AUX2_BASE.wrapping_add(x as u16), 0);

        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);
        assert_eq!(vm.bus().cio_channel_device(x), None);
        assert_eq!(
            vm.bus().cio_observations().back().unwrap().detail,
            "open passthrough"
        );

        vm.bus_mut()
            .ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(x as u16), CIO_COMMAND_GETREC);
        vm.bus_mut()
            .ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(x as u16), directory_record);
        vm.bus_mut()
            .ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(x as u16), 19);

        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);
        let record = (0..19)
            .map(|offset| vm.bus().ram().read(directory_record + offset))
            .collect::<Vec<_>>();
        assert!(record.windows(3).any(|window| window == b"DOS"));
        assert!(record.windows(3).any(|window| window == b"SYS"));
        assert_eq!(
            vm.bus().cio_observations().back().unwrap().detail,
            "read passthrough"
        );

        vm.bus_mut()
            .ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(x as u16), CIO_COMMAND_CLOSE);
        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);
        assert_eq!(
            vm.bus().cio_observations().back().unwrap().detail,
            "close passthrough"
        );
    }

    #[test]
    fn bundled_mydos_persists_a_file_on_a_copy_on_write_disk() {
        let mut vm = CompilerVm::default();
        vm.mount_bundled_mydos(1, DiskWritePolicy::CopyOnWrite)
            .unwrap();
        vm.prepare_execution_profile(ExecutionProfile::DiskBoot)
            .unwrap();
        vm.reset_cpu();

        for _ in 0..400_000 {
            if vm.bus().dos_boot_is_ready() && vm.cpu().registers().pc == 0xEA2D {
                break;
            }
            vm.step_cpu().unwrap();
        }
        assert_eq!(vm.cpu().registers().pc, 0xEA2D);

        let x = 0x10;
        let filename = 0x4200;
        let data = 0x4300;
        let text = b"HELLO FROM VM\x9B";
        vm.bus_mut()
            .ram_mut()
            .map(filename, b"D:VMTEST.TXT\x9B")
            .unwrap();
        vm.bus_mut()
            .ram_mut()
            .write(IOCB_DEVICE_BASE.wrapping_add(x as u16), 0xFF);
        configure_iocb(&mut vm, x, CIO_COMMAND_OPEN, filename, 0, 8);
        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);

        vm.bus_mut().ram_mut().map(data, text).unwrap();
        configure_iocb(&mut vm, x, CIO_COMMAND_PUTCHR, data, text.len() as u16, 0);
        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);
        configure_iocb(&mut vm, x, CIO_COMMAND_CLOSE, 0, 0, 0);
        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);

        assert!(vm.disk_is_dirty(1));
        assert!(!vm.dirty_disk_sectors(1).unwrap().is_empty());
        assert_eq!(
            vm.original_atr_bytes(1).unwrap(),
            BUNDLED_MYDOS_ATR.to_vec()
        );
        assert_ne!(vm.mounted_atr_bytes(1).unwrap(), BUNDLED_MYDOS_ATR.to_vec());
        assert!(vm.bus().sio_observations().iter().any(|observation| {
            matches!(
                observation.command,
                SIO_COMMAND_PUT_SECTOR | SIO_COMMAND_WRITE_SECTOR
            ) && observation.status == SIO_STATUS_SUCCESS
                && observation.bytes_transferred > 0
        }));

        vm.bus_mut()
            .ram_mut()
            .write(IOCB_DEVICE_BASE.wrapping_add(x as u16), 0xFF);
        configure_iocb(&mut vm, x, CIO_COMMAND_OPEN, filename, 0, 4);
        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);
        configure_iocb(&mut vm, x, CIO_COMMAND_GETREC, data, text.len() as u16, 0);
        let read_registers = run_native_ciov(&mut vm, x).unwrap();
        let actual = (0..text.len() as u16)
            .map(|offset| vm.bus().ram().read(data + offset))
            .collect::<Vec<_>>();
        // MyDOS returns the positive record-complete status $03 after GETREC;
        // the IOCB status and Y agree and the requested record includes EOL.
        assert_eq!(read_registers.y, 3);
        assert_eq!(vm.bus().ram().read(0x0343 + u16::from(x)), 3);
        assert_eq!(actual, text);
        configure_iocb(&mut vm, x, CIO_COMMAND_CLOSE, 0, 0, 0);
        assert_eq!(run_native_ciov(&mut vm, x).unwrap().y, 1);
    }

    #[test]
    fn bundled_mydos_handles_tn_file_and_subdirectory_mutations() {
        const XIO_RENAME: u8 = 32;
        const XIO_DELETE: u8 = 33;
        const XIO_MKDIR: u8 = 34;
        const XIO_LOCK: u8 = 35;
        const XIO_UNLOCK: u8 = 36;
        const XIO_BINARY_LOAD: u8 = 39;
        const XIO_CHDIR: u8 = 41;

        let mut vm = boot_bundled_mydos_to_prompt(DiskWritePolicy::CopyOnWrite);

        native_write_file(&mut vm, b"D1:LOCKED.TXT\x9B", b"LOCKED");
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_LOCK, b"D1:LOCKED.TXT\x9B", 0, 0),
            1
        );
        assert!(
            native_cio_filename_command(&mut vm, XIO_DELETE, b"D1:LOCKED.TXT\x9B", 0, 0) >= 0x80
        );
        assert_eq!(native_file_open_status(&mut vm, b"D1:LOCKED.TXT\x9B"), 1);
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_UNLOCK, b"D1:LOCKED.TXT\x9B", 0, 0),
            1
        );
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_DELETE, b"D1:LOCKED.TXT\x9B", 0, 0),
            1
        );
        assert!(native_file_open_status(&mut vm, b"D1:LOCKED.TXT\x9B") >= 0x80);

        native_write_file(&mut vm, b"D1:OLD.TXT\x9B", b"RENAMED CONTENT");
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_RENAME, b"D1:OLD.TXT,NEW.TXT\x9B", 0, 0,),
            1
        );
        assert!(native_file_open_status(&mut vm, b"D1:OLD.TXT\x9B") >= 0x80);
        assert_eq!(native_file_open_status(&mut vm, b"D1:NEW.TXT\x9B"), 1);

        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_MKDIR, b"D1:SUBDIR\x9B", 8, 0),
            1
        );
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_CHDIR, b"D1:SUBDIR\x9B", 0, 0),
            1
        );
        native_write_file(&mut vm, b"D:INNER.TXT\x9B", b"INSIDE");
        assert_eq!(
            native_read_file(&mut vm, b"D1:SUBDIR:INNER.TXT\x9B", 6),
            b"INSIDE"
        );
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_CHDIR, b"D1:\x9B", 0, 0),
            1
        );
        assert!(native_file_open_status(&mut vm, b"D1:INNER.TXT\x9B") >= 0x80);

        native_write_file(&mut vm, b"D1:A.TMP\x9B", b"A");
        native_write_file(&mut vm, b"D1:B.TMP\x9B", b"B");
        native_write_file(&mut vm, b"D1:KEEP.DAT\x9B", b"KEEP");
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_DELETE, b"D1:*.TMP\x9B", 0, 0),
            1
        );
        assert!(native_file_open_status(&mut vm, b"D1:A.TMP\x9B") >= 0x80);
        assert!(native_file_open_status(&mut vm, b"D1:B.TMP\x9B") >= 0x80);
        assert_eq!(native_file_open_status(&mut vm, b"D1:KEEP.DAT\x9B"), 1);

        let load_object = [
            0xFF, 0xFF, 0x00, 0x50, 0x05, 0x50, 0xA9, 0x42, 0x8D, 0xFF, 0x4F, 0x60, 0xE2, 0x02,
            0xE3, 0x02, 0x00, 0x50,
        ];
        native_write_file(&mut vm, b"D1:LOADME.COM\x9B", &load_object);
        vm.bus_mut().ram_mut().write(0x4FFF, 0);
        assert_eq!(
            native_cio_filename_command(&mut vm, XIO_BINARY_LOAD, b"D1:LOADME.COM\x9B", 4, 0,),
            1
        );
        assert_eq!(vm.bus().ram().read(0x4FFF), 0x42);

        assert!(vm.disk_is_dirty(1));
        assert!(vm.bus().sio_observations().iter().all(|observation| {
            observation.handled
                && matches!(
                    observation.status,
                    SIO_STATUS_SUCCESS | SIO_STATUS_DEVICE_TIMEOUT
                )
        }));
    }

    #[test]
    fn bundled_mydos_formats_and_remounts_a_copy_on_write_disk() {
        let mut vm = boot_bundled_mydos_to_prompt(DiskWritePolicy::CopyOnWrite);
        let contents = b"SURVIVES ATR EXPORT";

        native_write_file(&mut vm, b"D1:BEFORE.TXT\x9B", b"ERASE ME");
        vm.bus_mut().ram_mut().write(0x07C4, 0x02);
        vm.bus_mut().ram_mut().write(0x07CC, 0x10);
        let sio_before_format = vm.bus().sio_observations().len();

        assert_eq!(
            native_cio_filename_command(&mut vm, 254, b"D1:\x9B", 0, 0),
            1
        );

        let format = vm
            .bus()
            .sio_observations()
            .iter()
            .skip(sio_before_format)
            .find(|observation| {
                matches!(
                    observation.command,
                    SIO_COMMAND_FORMAT | SIO_COMMAND_FORMAT_ENHANCED
                )
            })
            .expect("MyDOS should issue an SIO format request");
        assert_eq!(format.unit, 1);
        assert_eq!(format.status, SIO_STATUS_SUCCESS);
        assert_eq!(format.bytes_transferred, 256);
        assert!(native_file_open_status(&mut vm, b"D1:BEFORE.TXT\x9B") >= 0x80);

        native_write_file(&mut vm, b"D1:AFTER.TXT\x9B", contents);
        assert_eq!(
            native_read_file(&mut vm, b"D1:AFTER.TXT\x9B", contents.len()),
            contents
        );
        let formatted = vm.mounted_atr_bytes(1).unwrap();

        let mut remounted = CompilerVm::default();
        remounted
            .mount_bundled_mydos(1, DiskWritePolicy::ReadOnly)
            .unwrap();
        remounted
            .mount_atr_bytes(2, formatted, DiskWritePolicy::ReadOnly)
            .unwrap();
        remounted
            .prepare_execution_profile(ExecutionProfile::DiskBoot)
            .unwrap();
        remounted.reset_cpu();
        for _ in 0..400_000 {
            if remounted.bus().dos_boot_is_ready() && remounted.cpu().registers().pc == 0xEA2D {
                break;
            }
            remounted.step_cpu().unwrap();
        }
        assert_eq!(remounted.cpu().registers().pc, 0xEA2D);
        assert_eq!(
            native_file_open_status(&mut remounted, b"D2:AFTER.TXT\x9B"),
            1
        );
        assert_eq!(
            native_read_file(&mut remounted, b"D2:AFTER.TXT\x9B", contents.len()),
            contents
        );
    }

    #[test]
    fn tn_standalone_renders_a_structurally_valid_main_screen() {
        let mut tn = TnHarness::boot_bundled(&[(1, DiskWritePolicy::ReadOnly)]);
        tn.launch();
        let screen = tn.vm.bus().text_screen_snapshot(40, 24);
        assert_eq!(screen.columns, 40);
        assert_eq!(screen.rows, 24);
        assert_eq!(screen.lines.len(), 24);
        assert!(screen.lines.iter().all(|line| line.len() == 40));

        // Keep the release fields flexible: both the version and copyright
        // year change independently of the main-screen layout.
        let release = screen.lines[0]
            .strip_prefix("Toms Navigator ")
            .and_then(|line| line.strip_suffix(" M.Kurcewicz"))
            .expect("TN title should retain its stable name and author");
        let (version, year) = release
            .split_once(" (c) ")
            .expect("TN title should separate version and copyright year");
        assert!(!version.is_empty());
        assert!(
            version
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'.')
        );
        assert_eq!(year.len(), 4);
        assert!(year.bytes().all(|byte| byte.is_ascii_digit()));

        let expected_body = [
            "D1:                                     ",
            ".   D1>         D- ..   D1>         D- .",
            "........................................",
            ".  DOS     .SYS.018..  DOS     .SYS.018.",
            ".  DUP     .SYS.027..  DUP     .SYS.027.",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            ".          .   .   ..          .   .   .",
            "........................................",
            ".663 FREE SECTORS  ..663 FREE SECTORS  .",
            "........................................",
            "New Copy Del Ren Atr View Mkdir Fmt Quit",
        ];
        let actual_body = screen.lines[1..]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(actual_body, expected_body);
    }

    #[test]
    fn tn_standalone_browses_views_and_copies_a_large_file_through_mydos() {
        const SOURCE_NAME: &[u8] = b"D1:AATEST.TXT\x9B";
        const DESTINATION_NAME: &[u8] = b"D2:AATEST.TXT\x9B";
        let mut tn = TnHarness::boot_bundled(&[
            (1, DiskWritePolicy::CopyOnWrite),
            (2, DiskWritePolicy::CopyOnWrite),
        ]);

        let mut contents = Vec::with_capacity(24_500);
        while contents.len() < 24_350 {
            contents.extend_from_slice(b"TN LARGE FILE EXERCISES BUFFERED CIO\x9B");
        }
        contents.extend_from_slice(b"VIEW-CONTENT-END\x9B");
        native_write_file(&mut tn.vm, SOURCE_NAME, &contents);
        tn.launch();
        tn.assert_screen_contains("AATEST");

        let tn_buffer = tn.vm.bus().ram().read_word(0x2C4A);
        let transfer_capacity = tn
            .vm
            .bus()
            .ram()
            .read_word(MEMTOP_OS_TOP_OF_FREE_MEMORY)
            .wrapping_sub(tn_buffer);
        assert!(
            contents.len() > usize::from(transfer_capacity),
            "fixture has {} bytes but TN can transfer {transfer_capacity} bytes per pass",
            contents.len()
        );

        tn.switch_panel(1);
        tn.select_drive(2);
        tn.switch_panel(0);

        tn.send_key(TN_VIEW_KEY, "TN file viewer");
        assert!(
            tn.vm
                .bus()
                .cio_channel0_output()
                .windows(b"VIEW-CONTENT-END".len())
                .any(|window| window == b"VIEW-CONTENT-END"),
            "captured {} byte(s)",
            tn.vm.bus().cio_channel0_output().len()
        );
        tn.send_text(b"\x9B", "return from TN file viewer");
        tn.assert_screen_contains("AATEST");

        tn.send_key(TN_COPY_KEY, "TN cross-drive copy");
        let copied = native_read_file(&mut tn.vm, DESTINATION_NAME, contents.len());
        assert_eq!(copied, contents);
        assert!(tn.vm.disk_is_dirty(2));
        assert!(tn.vm.bus().sio_observations().iter().all(|observation| {
            observation.handled
                && matches!(
                    observation.status,
                    SIO_STATUS_SUCCESS | SIO_STATUS_DEVICE_TIMEOUT
                )
        }));
    }

    #[test]
    fn tn_standalone_creates_d2_subdirectory_copies_and_renames_file() {
        const SOURCE_NAME: &[u8] = b"D1:ACOPY.TXT\x9B";
        const COPIED_NAME: &[u8] = b"D2:TARGET:ACOPY.TXT\x9B";
        const RENAMED_NAME: &[u8] = b"D2:TARGET:RENAMED.TXT\x9B";
        const CONTENTS: &[u8] = b"TN CROSS-DRIVE SUBDIRECTORY WORKFLOW";
        let mut tn = TnHarness::boot_bundled(&[
            (1, DiskWritePolicy::CopyOnWrite),
            (2, DiskWritePolicy::CopyOnWrite),
        ]);
        native_write_file(&mut tn.vm, SOURCE_NAME, CONTENTS);
        tn.launch();
        tn.assert_screen_contains("ACOPY");

        tn.switch_panel(1);
        tn.select_drive(2);
        tn.send_text(b"M", "TN subdirectory dialog");
        tn.assert_screen_contains("Subdirectory");
        tn.send_text(b"TARGET\x9B", "TN subdirectory creation");
        tn.assert_screen_contains("TARGET");
        tn.send_text(b"\x9B", "enter TN destination subdirectory");
        assert_eq!(tn.vm.bus().ram().read(TN_NEST_LEVEL), 1);
        assert_eq!(tn.vm.bus().ram().read(TN_FILE_COUNT), 0);

        tn.switch_panel(0);
        tn.send_key(TN_COPY_KEY, "TN subdirectory copy");

        tn.switch_panel(1);
        assert_eq!(tn.vm.bus().ram().read(TN_NEST_LEVEL), 1);
        // The destination panel was empty when it was first opened, before
        // the copy. Return to D2's root and re-enter it to make TN reread it.
        tn.send_text(b"\x1B", "leave TN destination subdirectory");
        assert_eq!(tn.vm.bus().ram().read(TN_NEST_LEVEL), 0);
        tn.assert_screen_contains("TARGET");
        tn.send_text(b"\x9B", "re-enter TN destination subdirectory");
        assert_eq!(tn.vm.bus().ram().read(TN_NEST_LEVEL), 1);
        assert_eq!(tn.vm.bus().ram().read(TN_FILE_COUNT), 1);

        tn.send_text(b"R", "TN rename dialog");
        tn.assert_screen_contains("Rename");
        tn.send_text(b"RENAMED.TXT\x9B", "TN rename completion");
        tn.assert_screen_contains("RENAMED");

        assert!(native_file_open_status(&mut tn.vm, COPIED_NAME) >= 0x80);
        assert_eq!(
            native_read_file(&mut tn.vm, RENAMED_NAME, CONTENTS.len()),
            CONTENTS
        );
        assert!(tn.vm.disk_is_dirty(2));
        assert!(tn.vm.bus().sio_observations().iter().all(|observation| {
            observation.handled
                && matches!(
                    observation.status,
                    SIO_STATUS_SUCCESS | SIO_STATUS_DEVICE_TIMEOUT
                )
        }));
    }

    #[test]
    fn tn_standalone_mutates_files_and_directories_through_its_ui() {
        let mut tn = TnHarness::boot_bundled(&[(1, DiskWritePolicy::CopyOnWrite)]);
        native_write_file(&mut tn.vm, b"D1:AOLD.TXT\x9B", b"TN UI MUTATION");
        native_write_file(&mut tn.vm, b"D1:BLOCK.TXT\x9B", b"ATTRIBUTE TARGET");
        tn.launch();
        tn.assert_screen_contains("AOLD");

        tn.send_text(b"R", "TN rename dialog");
        tn.assert_screen_contains("Rename");
        tn.send_text(b"ANEW.TXT\x9B", "TN rename completion");
        tn.assert_screen_contains("ANEW");

        tn.send_text(b"D", "TN delete confirmation");
        tn.assert_screen_contains("Delete");
        tn.send_text(b"D", "TN delete completion");
        tn.assert_screen_lacks("ANEW");

        tn.send_text(b"A", "TN attribute toggle");
        tn.assert_screen_contains("*BLOCK");
        tn.send_text(b"A", "TN attribute restore");
        tn.assert_screen_lacks("*BLOCK");

        tn.send_text(b"M", "TN subdirectory dialog");
        tn.assert_screen_contains("Subdirectory");
        tn.send_text(b"SUBDIR\x9B", "TN subdirectory creation");
        tn.assert_screen_contains("SUBDIR");
        tn.send_text(b"\x9B", "enter TN subdirectory");
        assert_eq!(tn.vm.bus().ram().read(TN_NEST_LEVEL), 1);
        tn.send_text(b"\x1B", "leave TN subdirectory");
        assert_eq!(tn.vm.bus().ram().read(TN_NEST_LEVEL), 0);

        assert!(native_file_open_status(&mut tn.vm, b"D1:ANEW.TXT\x9B") >= 0x80);
        assert_eq!(native_file_open_status(&mut tn.vm, b"D1:BLOCK.TXT\x9B"), 1);
        assert_eq!(
            native_cio_filename_command(&mut tn.vm, 41, b"D1:SUBDIR\x9B", 0, 0),
            1
        );
        assert!(tn.vm.disk_is_dirty(1));
    }

    #[test]
    fn explicit_images_override_bundled_action_environment() {
        let mut vm = CompilerVm::default();
        vm.load_image_bytes(
            ImageKind::Cartridge,
            "test-action.rom",
            DEFAULT_CART_BASE,
            vec![0xEA],
        )
        .unwrap();
        vm.load_image_bytes(ImageKind::Rom, "custom-os.rom", OS_ROM_BASE, vec![0xEA])
            .unwrap();
        vm.prepare_execution_profile(ExecutionProfile::OriginalCompiler)
            .unwrap();

        assert_eq!(vm.images().len(), 2);
        assert_eq!(vm.images()[0].path, PathBuf::from("test-action.rom"));
        assert_eq!(vm.images()[1].path, PathBuf::from("custom-os.rom"));
    }

    #[test]
    fn validates_loaded_images_for_execution_profiles() {
        let mut vm = CompilerVm::default();
        vm.validate_execution_profile(ExecutionProfile::StandaloneObject)
            .unwrap();
        vm.validate_execution_profile(ExecutionProfile::SyntheticTest)
            .unwrap();
        assert!(
            vm.validate_execution_profile(ExecutionProfile::DiskBoot)
                .unwrap_err()
                .contains("OS ROM")
        );
        assert!(
            vm.validate_execution_profile(ExecutionProfile::OriginalCompiler)
                .unwrap_err()
                .contains("cartridge")
        );

        vm.load_image_bytes(
            ImageKind::Cartridge,
            "action.rom",
            DEFAULT_CART_BASE,
            vec![0xEA],
        )
        .unwrap();
        assert!(
            vm.validate_execution_profile(ExecutionProfile::CartridgeObject)
                .unwrap_err()
                .contains("OS ROM")
        );

        vm.load_image_bytes(ImageKind::Rom, "atari-os.rom", OS_ROM_BASE, vec![0xEA])
            .unwrap();
        vm.validate_execution_profile(ExecutionProfile::OriginalCompiler)
            .unwrap();
        vm.validate_execution_profile(ExecutionProfile::CartridgeObject)
            .unwrap();
    }

    #[test]
    fn loads_and_runs_a_standalone_object_without_external_images() {
        let object = [
            0xFF, 0xFF, // object marker
            0x00, 0x30, 0x07, 0x30, // $3000-$3007
            0xA9, 0x42, // LDA #$42
            0x8D, 0x00, 0x06, // STA $0600
            0x4C, 0x05, 0x30, // JMP $3005
            0xE2, 0x02, 0xE3, 0x02, // RUNAD segment
            0x00, 0x30, // RUNAD=$3000
        ];
        let mut vm = CompilerVm::default();
        let load = vm
            .load_atari_object_for_execution(ExecutionProfile::StandaloneObject, &object)
            .unwrap();

        assert_eq!(load.run_address, Some(0x3000));
        assert_eq!(vm.cpu().registers().pc, 0x3000);
        assert_eq!(vm.cpu().registers().status, 0x24);
        let outcome = VmRunner::new(vm).run(RunRequest {
            max_steps: 10,
            stop_after_pc: Some(0x3005),
            ..RunRequest::default()
        });

        assert_eq!(outcome.stop_reason(), StopReason::PcReached { pc: 0x3005 });
        assert_eq!(outcome.report.completed_steps, 3);
        assert_eq!(outcome.memory().read(0x0600), 0x42);
        assert!(outcome.vm.images().is_empty());
    }

    #[test]
    fn object_loader_rejects_non_object_profiles_and_missing_runad() {
        let mut vm = CompilerVm::default();
        let error = vm
            .load_atari_object_for_execution(ExecutionProfile::SyntheticTest, &[0xFF, 0xFF])
            .unwrap_err();
        assert!(error.contains("not an Atari object execution profile"));

        let object_without_runad = [0xFF, 0xFF, 0x00, 0x30, 0x00, 0x30, 0xEA];
        let error = vm
            .load_atari_object_for_execution(
                ExecutionProfile::StandaloneObject,
                &object_without_runad,
            )
            .unwrap_err();
        assert!(error.contains("does not contain RUNAD"));
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
        assert_eq!(
            bus.read(BRKKEY_BREAK_KEY_FLAG),
            DEFAULT_HEADLESS_BRKKEY_NOT_PRESSED
        );
        assert_eq!(
            bus.read(RAMTOP_MEMORY_TOP_PAGE),
            DEFAULT_HEADLESS_RAMTOP_PAGE
        );
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
    fn portb_ddr_writes_do_not_change_memory_management_latch() {
        let mut bus = Bus::default();
        let mut os_rom = vec![0xAA; 0x4000];
        os_rom[0x1000] = 0x4C;
        bus.map_os_rom(0xC000, os_rom).unwrap();

        bus.write(PBCTL_PORTB_CONTROL, 0xFB);
        bus.write(PORTB, 0x7D);
        assert_eq!(bus.read(PORTB), 0x7D);
        assert_eq!(bus.io().portb(), 0xFF);
        assert_eq!(bus.visible_region(0x5000), BusRegion::Ram);
        assert_eq!(bus.read(0x5000), 0x00);

        bus.write(PBCTL_PORTB_CONTROL, 0xFF);
        assert_eq!(bus.read(PORTB), 0xFF);
        bus.write(PBCTL_PORTB_CONTROL, 0xFB);
        bus.write(PORTB, 0xFF);
        bus.write(PBCTL_PORTB_CONTROL, 0xFF);
        bus.write(PORTB, 0x7D);
        assert_eq!(bus.io().portb(), 0x7D);
        assert_eq!(bus.visible_region(0x5000), BusRegion::SelfTestRom);
        assert_eq!(bus.read(0x5000), 0x4C);
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
        assert!(!bus.scripted_cio_input_is_idle());

        bus.ram_mut().write(CH_KEY_CODE, 0xFF);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x70;
        cpu.registers.sp = 0xFB;

        assert!(!cpu.try_emulate_ciov(&mut bus));
        assert!(bus.scripted_cio_input_is_idle());

        bus.queue_scripted_cio_input_byte(b'R');
        assert!(!bus.scripted_cio_input_is_idle());
    }

    #[test]
    fn cpu_reads_scripted_cio_input_as_a_record() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x20), CIO_COMMAND_GETREC);
        bus.ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(0x20), 0x3000);
        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x20), 8);
        bus.cio_harness_devices[2] = Some(CioHarnessDevice::QueuedInput);
        bus.queue_scripted_cio_input_bytes(b"123\x9Bnext");
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x20;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers().pc, 0x2000);
        assert_eq!(cpu.registers().y, 0x01);
        assert_eq!(bus.ram().read_word(IOCB_LENGTH_BASE + 0x20), 4);
        assert_eq!(
            (0..4)
                .map(|offset| bus.ram().read(0x3000 + offset))
                .collect::<Vec<_>>(),
            b"123\x9B"
        );
        assert_eq!(bus.scripted_cio_input.front(), Some(&b'n'));
        assert_eq!(bus.cio_summary().bytes_read, 4);
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
    fn cpu_treats_closing_empty_cio_channel_as_success() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_CLOSE);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x10;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.pc, 0x2000);
        assert_eq!(cpu.registers.y, 0x01);
        assert_eq!(bus.cio_channel_device(0x10), None);
        assert_eq!(bus.cio_summary.closes, 1);
    }

    #[test]
    fn cpu_passes_unowned_close_to_native_os_when_requested() {
        let mut bus = Bus::default();
        bus.set_cio_fallback_policy(CioFallbackPolicy::NativeOs);
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_CLOSE);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x10;
        cpu.registers.sp = 0xFB;

        assert!(!cpu.try_emulate_ciov(&mut bus));
        assert_eq!(cpu.registers.pc, CIOV);
        assert_eq!(bus.cio_channel_device(0x10), None);
        assert_eq!(
            bus.cio_observations().back().unwrap().detail,
            "close passthrough"
        );
    }

    #[test]
    fn cpu_reads_host_getchr_blocks_without_text_translation() {
        let mut bus = Bus::default();
        bus.add_host_file("DATA.BIN", vec![0x00, b'\n', 0xFF, 0x42, 0x43]);
        bus.cio_harness_devices[1] = Some(CioHarnessDevice::Host {
            file_index: 0,
            offset: 0,
        });
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_GETCHR);
        bus.ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(0x10), 0x3000);
        let mut cpu = Cpu::default();
        cpu.registers.x = 0x10;

        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x10), 3);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = CIOV;
        cpu.registers.sp = 0xFB;
        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.y, 0x01);
        assert_eq!(bus.ram().read_word(IOCB_LENGTH_BASE + 0x10), 3);
        assert_eq!(
            (0..3)
                .map(|offset| bus.ram().read(0x3000 + offset))
                .collect::<Vec<_>>(),
            vec![0x00, b'\n', 0xFF]
        );

        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x10), 3);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = CIOV;
        cpu.registers.sp = 0xFB;
        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.y, 0x88);
        assert_eq!(bus.ram().read_word(IOCB_LENGTH_BASE + 0x10), 2);
        assert_eq!(bus.ram().read(0x3000), 0x42);
        assert_eq!(bus.ram().read(0x3001), 0x43);
        assert_eq!(
            bus.cio_channel_device(0x10),
            Some(CioHarnessDevice::Host {
                file_index: 0,
                offset: 5
            })
        );
    }

    #[test]
    fn zero_length_host_getchr_remains_a_translated_character_read() {
        let mut bus = Bus::default();
        bus.add_host_file("TEXT.ACT", b"\n".to_vec());
        bus.cio_harness_devices[1] = Some(CioHarnessDevice::Host {
            file_index: 0,
            offset: 0,
        });
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_GETCHR);
        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x10), 0);
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x10;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(cpu.registers.a, ATASCII_EOL);
        assert_eq!(cpu.registers.y, 0x01);
        assert_eq!(bus.ram().read_word(IOCB_LENGTH_BASE + 0x10), 0);
    }

    #[test]
    fn cpu_writes_harness_host_output() {
        let mut bus = Bus::default();
        bus.add_host_output("OUT.COM");
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x10), CIO_COMMAND_OPEN);
        bus.ram_mut().write(IOCB_AUX1_BASE.wrapping_add(0x10), 0x08);
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
        bus.ram_mut()
            .map(0x3100, &[0xFF, 0xFF, 0x00, 0x30])
            .unwrap();
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x10;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(
            bus.host_file_bytes("OUT.COM"),
            Some(&[0xFF, 0xFF, 0x00, 0x30][..])
        );
    }

    #[test]
    fn cpu_opens_harness_host_files_through_d_device() {
        let mut bus = Bus::default();
        bus.add_host_file("LIB.ACT", b"BYTE x\n".to_vec());
        bus.ram_mut()
            .write(IOCB_COMMAND_BASE.wrapping_add(0x30), CIO_COMMAND_OPEN);
        bus.ram_mut()
            .write_word(IOCB_BUFFER_BASE.wrapping_add(0x30), 0x3000);
        bus.ram_mut()
            .write_word(IOCB_LENGTH_BASE.wrapping_add(0x30), 9);
        bus.ram_mut().map(0x3000, b"D:LIB.ACT").unwrap();
        bus.ram_mut().write(0x01FC, 0xFF);
        bus.ram_mut().write(0x01FD, 0x1F);
        let mut cpu = Cpu::default();
        cpu.registers.pc = CIOV;
        cpu.registers.x = 0x30;
        cpu.registers.sp = 0xFB;

        cpu.step(&mut bus).unwrap();

        assert_eq!(
            bus.cio_channel_device(0x30),
            Some(CioHarnessDevice::Host {
                file_index: 0,
                offset: 0
            })
        );
    }

    #[test]
    fn host_note_and_point_round_trip_the_file_cursor() {
        let mut bus = Bus::default();
        bus.add_host_file("DATA.BIN", vec![0; 400]);
        bus.cio_harness_devices[1] = Some(CioHarnessDevice::Host {
            file_index: 0,
            offset: 300,
        });

        assert_eq!(bus.note_host_position(0x10), Some(300));
        assert_eq!(bus.ram().read(IOCB_AUX3_BASE + 0x10), 1);
        assert_eq!(bus.ram().read(IOCB_AUX4_BASE + 0x10), 0);
        assert_eq!(bus.ram().read(IOCB_AUX5_BASE + 0x10), 44);

        bus.cio_harness_devices[1] = Some(CioHarnessDevice::Host {
            file_index: 0,
            offset: 0,
        });
        assert_eq!(bus.point_host_position(0x10), Some(300));
        assert_eq!(
            bus.cio_channel_device(0x10),
            Some(CioHarnessDevice::Host {
                file_index: 0,
                offset: 300
            })
        );
    }

    #[test]
    fn headless_screen_supports_pixel_line_and_fill_operations() {
        let mut bus = Bus::default();
        bus.cio_harness_devices[6] = Some(CioHarnessDevice::Screen);
        bus.ram_mut().write_word(COLCRS, 10);
        bus.ram_mut().write(ROWCRS, 20);

        assert_eq!(bus.write_screen_bytes_for_iocb(0x60, 3, false), Some(1));
        assert_eq!(bus.graphics_pixel(10, 20), 3);
        assert_eq!(bus.read_screen_pixel(), 3);

        bus.ram_mut().write_word(COLCRS, 12);
        bus.ram_mut().write(ROWCRS, 22);
        bus.ram_mut().write(GRAPHICS_FILL_COLOR, 5);
        assert!(bus.draw_screen_to_cursor(0x60, false));
        assert_eq!(bus.graphics_pixel(11, 21), 5);
        assert_eq!(bus.graphics_pixel(12, 22), 5);

        bus.ram_mut().write_word(COLCRS, 14);
        bus.ram_mut().write(ROWCRS, 24);
        bus.ram_mut().write(GRAPHICS_FILL_COLOR, 7);
        assert!(bus.draw_screen_to_cursor(0x60, true));
        assert_eq!(bus.graphics_pixel(13, 23), 7);
        assert_eq!(bus.graphics_pixel(14, 24), 7);
    }

    #[test]
    fn cpu_captures_channel_zero_cio_output() {
        let mut bus = Bus::default();
        bus.ram_mut().write(IOCB_COMMAND_BASE, CIO_COMMAND_PUTREC);
        bus.ram_mut().write_word(IOCB_BUFFER_BASE, 0x3000);
        bus.ram_mut().write_word(IOCB_LENGTH_BASE, 2);
        bus.ram_mut().map(0x3000, b"OK").unwrap();
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

    fn packed_bcd(value: u16) -> u8 {
        (((value / 10) << 4) | (value % 10)) as u8
    }

    #[test]
    fn cpu_decimal_adc_produces_valid_packed_bcd_results() {
        for lhs in 0..=99 {
            for rhs in 0..=99 {
                for carry_in in [false, true] {
                    let mut cpu = Cpu::default();
                    cpu.registers.a = packed_bcd(lhs);
                    cpu.set_flag(StatusFlags::DECIMAL, true);
                    cpu.set_flag(StatusFlags::CARRY, carry_in);

                    cpu.adc(packed_bcd(rhs));

                    let sum = lhs + rhs + u16::from(carry_in);
                    assert_eq!(
                        cpu.registers.a,
                        packed_bcd(sum % 100),
                        "{lhs:02} + {rhs:02} + {}",
                        u8::from(carry_in)
                    );
                    assert_eq!(
                        cpu.flag(StatusFlags::CARRY),
                        sum >= 100,
                        "carry for {lhs:02} + {rhs:02} + {}",
                        u8::from(carry_in)
                    );
                }
            }
        }
    }

    #[test]
    fn cpu_decimal_sbc_produces_valid_packed_bcd_results() {
        for lhs in 0..=99 {
            for rhs in 0..=99 {
                for carry_in in [false, true] {
                    let mut cpu = Cpu::default();
                    cpu.registers.a = packed_bcd(lhs);
                    cpu.set_flag(StatusFlags::DECIMAL, true);
                    cpu.set_flag(StatusFlags::CARRY, carry_in);

                    cpu.sbc(packed_bcd(rhs));

                    let difference = lhs as i16 - rhs as i16 - i16::from(!carry_in);
                    let wrapped = difference.rem_euclid(100) as u16;
                    assert_eq!(
                        cpu.registers.a,
                        packed_bcd(wrapped),
                        "{lhs:02} - {rhs:02} - {}",
                        u8::from(!carry_in)
                    );
                    assert_eq!(
                        cpu.flag(StatusFlags::CARRY),
                        difference >= 0,
                        "carry for {lhs:02} - {rhs:02} - {}",
                        u8::from(!carry_in)
                    );
                }
            }
        }
    }

    #[test]
    fn cpu_decimal_arithmetic_preserves_nmos_flag_behavior() {
        let mut cpu = Cpu::default();
        cpu.registers.a = 0x50;
        cpu.set_flag(StatusFlags::DECIMAL, true);
        cpu.set_flag(StatusFlags::CARRY, false);

        cpu.adc(0x50);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.flag(StatusFlags::CARRY));
        assert!(cpu.flag(StatusFlags::NEGATIVE));
        assert!(cpu.flag(StatusFlags::OVERFLOW));
        assert!(!cpu.flag(StatusFlags::ZERO));

        cpu.registers.a = 0x80;
        cpu.set_flag(StatusFlags::CARRY, true);
        cpu.sbc(0x01);

        assert_eq!(cpu.registers.a, 0x79);
        assert!(cpu.flag(StatusFlags::CARRY));
        assert!(!cpu.flag(StatusFlags::NEGATIVE));
        assert!(cpu.flag(StatusFlags::OVERFLOW));
        assert!(!cpu.flag(StatusFlags::ZERO));

        cpu.registers.a = 0x00;
        cpu.set_flag(StatusFlags::CARRY, true);
        cpu.sbc(0x00);

        assert_eq!(cpu.registers.a, 0x00);
        assert!(cpu.flag(StatusFlags::CARRY));
        assert!(cpu.flag(StatusFlags::ZERO));
    }

    #[test]
    fn bundled_atari_os_converts_integer_to_ascii() {
        const INTEGER_TO_FLOAT: u16 = 0xD9AA;
        const FLOAT_TO_ASCII: u16 = 0xD8E6;
        const FLOAT_REGISTER: u16 = 0x00D4;
        const ASCII_BUFFER_POINTER: u16 = 0x00F3;
        const PROGRAM_START: u16 = 0x0200;
        const PROGRAM_END: u16 = 0x020E;

        let mut bus = Bus::default();
        bus.map_os_rom(OS_ROM_BASE, BUNDLED_ALTIRRA_OS.to_vec())
            .unwrap();
        bus.ram_mut()
            .map(
                PROGRAM_START,
                &[
                    0xA9,
                    0xD2, // LDA #<$04D2
                    0x85,
                    FLOAT_REGISTER as u8, // STA FR0
                    0xA9,
                    0x04, // LDA #>$04D2
                    0x85,
                    FLOAT_REGISTER as u8 + 1, // STA FR0+1
                    0x20,
                    INTEGER_TO_FLOAT as u8,
                    (INTEGER_TO_FLOAT >> 8) as u8,
                    0x20,
                    FLOAT_TO_ASCII as u8,
                    (FLOAT_TO_ASCII >> 8) as u8,
                ],
            )
            .unwrap();
        let mut cpu = Cpu::default();
        cpu.registers.pc = PROGRAM_START;

        for _ in 0..50_000 {
            if cpu.registers.pc == PROGRAM_END {
                break;
            }
            cpu.step(&mut bus).unwrap();
        }
        assert_eq!(cpu.registers.pc, PROGRAM_END);

        let buffer = bus.ram().read_word(ASCII_BUFFER_POINTER);
        let mut ascii = Vec::new();
        for offset in 0..16 {
            let byte = bus.read(buffer.wrapping_add(offset));
            ascii.push(byte & 0x7F);
            if byte & 0x80 != 0 {
                break;
            }
        }
        assert_eq!(ascii, b"1234");
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
    fn cpu_traps_writes_to_protected_code_ranges() {
        let mut bus = Bus::default();
        bus.ram_mut()
            .map(
                0x0200,
                &[
                    0xA9, 0x42, // LDA #$42
                    0x8D, 0x05, 0x30, // STA $3005
                ],
            )
            .unwrap();
        bus.ram_mut().write(0x3005, 0xEA);
        bus.ram_mut().write(0xFFFC, 0x00);
        bus.ram_mut().write(0xFFFD, 0x02);
        bus.add_protected_code_range(AddressRange {
            start: 0x3000,
            end: 0x30FF,
        });
        let mut cpu = Cpu::default();
        cpu.reset(&mut bus);

        cpu.step(&mut bus).unwrap();
        assert_eq!(
            cpu.step(&mut bus).unwrap_err(),
            CpuError::ProtectedCodeWrite {
                pc: 0x0202,
                address: 0x3005,
                old_value: 0xEA,
                new_value: 0x42,
                region: BusRegion::Ram,
            }
        );
        assert_eq!(bus.read(0x3005), 0xEA);
        assert!(cpu.halted());
    }

    #[test]
    fn cpu_decodes_all_legal_nmos_6502_opcodes() {
        const LEGAL_NMOS_6502_OPCODES: &[u8] = &[
            0x00, 0x01, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0D, 0x0E, 0x10, 0x11, 0x15, 0x16, 0x18,
            0x19, 0x1D, 0x1E, 0x20, 0x21, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2A, 0x2C, 0x2D, 0x2E,
            0x30, 0x31, 0x35, 0x36, 0x38, 0x39, 0x3D, 0x3E, 0x40, 0x41, 0x45, 0x46, 0x48, 0x49,
            0x4A, 0x4C, 0x4D, 0x4E, 0x50, 0x51, 0x55, 0x56, 0x58, 0x59, 0x5D, 0x5E, 0x60, 0x61,
            0x65, 0x66, 0x68, 0x69, 0x6A, 0x6C, 0x6D, 0x6E, 0x70, 0x71, 0x75, 0x76, 0x78, 0x79,
            0x7D, 0x7E, 0x81, 0x84, 0x85, 0x86, 0x88, 0x8A, 0x8C, 0x8D, 0x8E, 0x90, 0x91, 0x94,
            0x95, 0x96, 0x98, 0x99, 0x9A, 0x9D, 0xA0, 0xA1, 0xA2, 0xA4, 0xA5, 0xA6, 0xA8, 0xA9,
            0xAA, 0xAC, 0xAD, 0xAE, 0xB0, 0xB1, 0xB4, 0xB5, 0xB6, 0xB8, 0xB9, 0xBA, 0xBC, 0xBD,
            0xBE, 0xC0, 0xC1, 0xC4, 0xC5, 0xC6, 0xC8, 0xC9, 0xCA, 0xCC, 0xCD, 0xCE, 0xD0, 0xD1,
            0xD5, 0xD6, 0xD8, 0xD9, 0xDD, 0xDE, 0xE0, 0xE1, 0xE4, 0xE5, 0xE6, 0xE8, 0xE9, 0xEA,
            0xEC, 0xED, 0xEE, 0xF0, 0xF1, 0xF5, 0xF6, 0xF8, 0xF9, 0xFD, 0xFE,
        ];

        for opcode in LEGAL_NMOS_6502_OPCODES {
            let mut bus = Bus::default();
            bus.ram_mut()
                .map(0x0200, &opcode_probe_program(*opcode))
                .unwrap();
            bus.ram_mut().write_word(0x0040, 0x3000);
            bus.ram_mut().write_word(0x0042, 0x3000);
            bus.ram_mut().write_word(0x0340, 0x3000);
            bus.ram_mut().write(0x3000, 0x42);
            bus.ram_mut().write(0x3003, 0x42);
            bus.ram_mut().write(0xFFFE, 0x34);
            bus.ram_mut().write(0xFFFF, 0x12);
            bus.ram_mut().write(0x01FE, 0x20);
            bus.ram_mut().write(0x01FF, 0x78);
            bus.ram_mut().write(0x0100, 0x56);

            let mut cpu = Cpu::default();
            cpu.registers.pc = 0x0200;
            cpu.registers.sp = 0xFD;
            cpu.registers.a = 0x55;
            cpu.registers.x = 0x02;
            cpu.registers.y = 0x03;

            let result = cpu.step(&mut bus);

            assert!(
                !matches!(result, Err(CpuError::UnsupportedOpcode { .. })),
                "legal opcode ${opcode:02X} should decode"
            );
        }
    }

    fn opcode_probe_program(opcode: u8) -> Vec<u8> {
        let operand_len = match opcode {
            0x0A | 0x18 | 0x28 | 0x2A | 0x38 | 0x40 | 0x48 | 0x4A | 0x58 | 0x60 | 0x68 | 0x6A
            | 0x78 | 0x88 | 0x8A | 0x98 | 0x9A | 0xA8 | 0xAA | 0xB8 | 0xBA | 0xC8 | 0xCA | 0xD8
            | 0xE8 | 0xEA | 0xF8 => 0,
            0x0D | 0x0E | 0x19 | 0x1D | 0x1E | 0x20 | 0x2C | 0x2D | 0x2E | 0x39 | 0x3D | 0x3E
            | 0x4C | 0x4D | 0x4E | 0x59 | 0x5D | 0x5E | 0x6C | 0x6D | 0x6E | 0x79 | 0x7D | 0x7E
            | 0x8C | 0x8D | 0x8E | 0x99 | 0x9D | 0xAC | 0xAD | 0xAE | 0xB9 | 0xBC | 0xBD | 0xBE
            | 0xCC | 0xCD | 0xCE | 0xD9 | 0xDD | 0xDE | 0xEC | 0xED | 0xEE | 0xF9 | 0xFD | 0xFE => {
                2
            }
            _ => 1,
        };
        let mut program = vec![opcode];
        match operand_len {
            0 => {}
            1 => program.push(0x40),
            2 => program.extend_from_slice(&[0x40, 0x03]),
            _ => unreachable!(),
        }
        program
    }

    #[test]
    fn decodes_action_symbol_tables_from_official_table_shape() {
        let mut memory = Memory::default();
        memory.write_word(ACTION_GLOBAL_SYMBOL_TABLE_POINTER, 0x2000);
        memory.write(0x2001, 0x30);
        memory.write(0x2101, 0x00);
        write_symbol_entry(&mut memory, 0x3000, "Plot", 0xC0, Some(0xA6C3), &[4, 2]);

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

        let json =
            format_action_symbol_dump_json(&decode_action_symbol_tables_from_memory(&memory));

        assert!(json.contains("\"global_index\": \"$2000\""));
        assert!(json.contains("\"name\":\"Main\""));
        assert!(json.contains("\"address\":\"$316C\""));
        assert!(json.contains("\"locals\": []"));
    }

    #[test]
    fn loads_atari_object_segments_and_runad() {
        let mut memory = Memory::default();
        let object = [
            0xFF, 0xFF, 0x00, 0x30, 0x02, 0x30, 0xA9, 0x01, 0x60, 0xE2, 0x02, 0xE3, 0x02, 0x00,
            0x30,
        ];

        let report = load_atari_object_into_memory(&mut memory, &object).unwrap();

        assert_eq!(report.run_address, Some(0x3000));
        assert_eq!(
            report.segments,
            vec![
                AtariLoadSegment {
                    start: 0x3000,
                    end: 0x3002,
                    len: 3
                },
                AtariLoadSegment {
                    start: RUNAD,
                    end: RUNAD + 1,
                    len: 2
                }
            ]
        );
        assert_eq!(memory.read(0x3000), 0xA9);
        assert_eq!(memory.read(0x3001), 0x01);
        assert_eq!(memory.read(0x3002), 0x60);
        assert_eq!(memory.read_word(RUNAD), 0x3000);
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
