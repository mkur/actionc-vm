use std::path::PathBuf;

use crate::memory::mapped_end;
use crate::{CAR_HEADER_SIZE, CAR_MAGIC, Cartridge};

pub(crate) const BUNDLED_ALTIRRA_OS_LABEL: &str = "embedded:altirraos-xl.rom";
pub(crate) const BUNDLED_ALTIRRA_OS: &[u8] = include_bytes!("../roms/altirraos-xl.rom");

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

impl LoadedImage {
    pub(crate) fn prepare(
        kind: ImageKind,
        path: PathBuf,
        base: u16,
        bytes: Vec<u8>,
    ) -> Result<Self, String> {
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

pub(crate) fn checksum16(bytes: &[u8]) -> u16 {
    bytes
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(*byte as u16))
}

pub(crate) fn crc32(bytes: &[u8]) -> u32 {
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
