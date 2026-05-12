use std::fs;
use std::path::PathBuf;

pub const RAM_SIZE: usize = 0x10000;
pub const DEFAULT_CART_BASE: u16 = 0xA000;
pub const OS_ROM_BASE: u16 = 0xC000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmConfig {
    pub cartridge: Option<PathBuf>,
    pub cartridge_base: u16,
    pub os_rom: Option<PathBuf>,
    pub source: Option<PathBuf>,
    pub extra_images: Vec<(ImageKind, PathBuf, u16)>,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            cartridge: None,
            cartridge_base: DEFAULT_CART_BASE,
            os_rom: None,
            source: None,
            extra_images: Vec::new(),
        }
    }
}

impl VmConfig {
    pub fn load(&self) -> Result<CompilerVm, String> {
        let mut vm = CompilerVm::default();

        if let Some(path) = &self.cartridge {
            vm.load_image(ImageKind::Cartridge, path.clone(), self.cartridge_base)?;
        }

        if let Some(path) = &self.os_rom {
            vm.load_image(ImageKind::Rom, path.clone(), OS_ROM_BASE)?;
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
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerVm {
    memory: Memory,
    images: Vec<LoadedImage>,
    source: Option<Vec<u8>>,
}

impl Default for CompilerVm {
    fn default() -> Self {
        Self {
            memory: Memory::default(),
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
        &self.memory
    }

    pub fn source(&self) -> Option<&[u8]> {
        self.source.as_deref()
    }

    fn load_image(&mut self, kind: ImageKind, path: PathBuf, base: u16) -> Result<(), String> {
        let bytes = fs::read(&path)
            .map_err(|err| format!("failed to read image `{}`: {err}", path.display()))?;
        self.memory.map(base, &bytes)?;
        self.images.push(LoadedImage {
            kind,
            path,
            base,
            bytes,
        });
        Ok(())
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
        let start = base as usize;
        let end = start
            .checked_add(bytes.len())
            .ok_or_else(|| "image mapping overflows address space".to_string())?;

        if end > RAM_SIZE {
            return Err(format!(
                "image at ${base:04X} with {} byte(s) exceeds 64K address space",
                bytes.len()
            ));
        }

        self.bytes[start..end].copy_from_slice(bytes);
        Ok(())
    }
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
}
