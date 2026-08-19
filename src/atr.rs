use std::collections::BTreeSet;

const ATR_HEADER_SIZE: usize = 16;
const ATR_MAGIC: u16 = 0x0296;
const BOOT_SECTOR_COUNT: usize = 3;
const BOOT_SECTOR_SIZE: usize = 128;

pub static BUNDLED_MYDOS_ATR: &[u8] = include_bytes!("../disks/mydos.atr");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskWritePolicy {
    ReadOnly,
    CopyOnWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountedDisk {
    pub unit: u8,
    pub image: AtrImage,
    pub write_policy: DiskWritePolicy,
}

/// A validated, mutable ATR disk image.
///
/// Sector numbers are one-based, as they are in Atari SIO. On 256-byte ATRs,
/// the first three boot sectors remain 128 bytes long.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtrImage {
    bytes: Vec<u8>,
    original_bytes: Vec<u8>,
    sector_size: usize,
    sector_count: usize,
    dirty_sectors: BTreeSet<u16>,
}

impl AtrImage {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, String> {
        let bytes = bytes.into();
        if bytes.len() < ATR_HEADER_SIZE {
            return Err("file is too small to be an ATR image".to_string());
        }
        if u16::from_le_bytes([bytes[0], bytes[1]]) != ATR_MAGIC {
            return Err("missing ATR magic $0296".to_string());
        }

        let sector_size = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
        if !matches!(sector_size, 128 | 256) {
            return Err(format!("unsupported ATR sector size {sector_size}"));
        }

        let declared_paragraphs =
            usize::from(u16::from_le_bytes([bytes[2], bytes[3]])) | (usize::from(bytes[6]) << 16);
        let declared_payload = declared_paragraphs
            .checked_mul(16)
            .ok_or_else(|| "ATR payload length overflows this host".to_string())?;
        let actual_payload = bytes.len() - ATR_HEADER_SIZE;
        if declared_payload != actual_payload {
            return Err(format!(
                "ATR header declares {declared_payload} payload byte(s), but image contains {actual_payload}"
            ));
        }

        let sector_count = sector_count_for_payload(actual_payload, sector_size)?;
        if sector_count == 0 {
            return Err("ATR image contains no sectors".to_string());
        }

        let original_bytes = bytes.clone();
        Ok(Self {
            bytes,
            original_bytes,
            sector_size,
            sector_count,
            dirty_sectors: BTreeSet::new(),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn sector_size(&self) -> usize {
        self.sector_size
    }

    pub fn sector_count(&self) -> usize {
        self.sector_count
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty_sectors.is_empty()
    }

    pub fn dirty_sectors(&self) -> Vec<u16> {
        self.dirty_sectors.iter().copied().collect()
    }

    pub fn original_bytes(&self) -> &[u8] {
        &self.original_bytes
    }

    pub fn clear_dirty(&mut self) {
        self.original_bytes.clone_from(&self.bytes);
        self.dirty_sectors.clear();
    }

    pub fn sector_len(&self, sector: u16) -> Result<usize, String> {
        self.sector_range(sector).map(|(_, len)| len)
    }

    pub fn read_sector(&self, sector: u16) -> Result<&[u8], String> {
        let (start, len) = self.sector_range(sector)?;
        Ok(&self.bytes[start..start + len])
    }

    pub fn write_sector(&mut self, sector: u16, data: &[u8]) -> Result<(), String> {
        let (start, len) = self.sector_range(sector)?;
        if data.len() != len {
            return Err(format!(
                "ATR sector {sector} requires {len} byte(s), got {}",
                data.len()
            ));
        }
        if self.bytes[start..start + len] != *data {
            self.bytes[start..start + len].copy_from_slice(data);
        }
        if self.original_bytes[start..start + len] == *data {
            self.dirty_sectors.remove(&sector);
        } else {
            self.dirty_sectors.insert(sector);
        }
        Ok(())
    }

    fn sector_range(&self, sector: u16) -> Result<(usize, usize), String> {
        if sector == 0 {
            return Err("ATR sector numbers start at one".to_string());
        }
        if usize::from(sector) > self.sector_count {
            return Err(format!(
                "ATR sector {sector} is outside image range 1..={}",
                self.sector_count
            ));
        }

        let index = usize::from(sector) - 1;
        if self.sector_size == BOOT_SECTOR_SIZE || index < BOOT_SECTOR_COUNT {
            Ok((ATR_HEADER_SIZE + index * BOOT_SECTOR_SIZE, BOOT_SECTOR_SIZE))
        } else {
            Ok((
                ATR_HEADER_SIZE
                    + BOOT_SECTOR_COUNT * BOOT_SECTOR_SIZE
                    + (index - BOOT_SECTOR_COUNT) * self.sector_size,
                self.sector_size,
            ))
        }
    }
}

fn sector_count_for_payload(payload: usize, sector_size: usize) -> Result<usize, String> {
    if sector_size == BOOT_SECTOR_SIZE {
        if payload % BOOT_SECTOR_SIZE != 0 {
            return Err(format!(
                "128-byte ATR payload length {payload} is not sector aligned"
            ));
        }
        return Ok(payload / BOOT_SECTOR_SIZE);
    }

    let boot_bytes = BOOT_SECTOR_COUNT * BOOT_SECTOR_SIZE;
    if payload <= boot_bytes {
        if payload % BOOT_SECTOR_SIZE != 0 {
            return Err(format!(
                "ATR boot payload length {payload} is not 128-byte sector aligned"
            ));
        }
        return Ok(payload / BOOT_SECTOR_SIZE);
    }

    let body = payload - boot_bytes;
    if body % sector_size != 0 {
        return Err(format!(
            "256-byte ATR payload has {body} byte(s) after its boot sectors, which is not sector aligned"
        ));
    }
    Ok(BOOT_SECTOR_COUNT + body / sector_size)
}

#[cfg(test)]
mod tests {
    use super::AtrImage;

    fn atr_bytes(sector_size: usize, sectors: usize) -> Vec<u8> {
        let payload = if sector_size == 128 {
            sectors * 128
        } else {
            sectors.min(3) * 128 + sectors.saturating_sub(3) * 256
        };
        assert_eq!(payload % 16, 0);
        let paragraphs = payload / 16;
        let mut bytes = vec![0; 16 + payload];
        bytes[0..2].copy_from_slice(&0x0296u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&(paragraphs as u16).to_le_bytes());
        bytes[4..6].copy_from_slice(&(sector_size as u16).to_le_bytes());
        bytes[6] = (paragraphs >> 16) as u8;
        bytes
    }

    #[test]
    fn rejects_bad_headers_lengths_and_geometry() {
        assert!(AtrImage::from_bytes(vec![0; 15]).is_err());

        let mut bad_magic = atr_bytes(128, 1);
        bad_magic[0] = 0;
        assert!(AtrImage::from_bytes(bad_magic).is_err());

        let mut unsupported = atr_bytes(128, 1);
        unsupported[4..6].copy_from_slice(&512u16.to_le_bytes());
        assert!(AtrImage::from_bytes(unsupported).is_err());

        let mut wrong_length = atr_bytes(128, 1);
        wrong_length.push(0);
        assert!(AtrImage::from_bytes(wrong_length).is_err());

        let mut unaligned = atr_bytes(256, 4);
        unaligned.extend_from_slice(&[0; 16]);
        let paragraphs = (unaligned.len() - 16) / 16;
        unaligned[2..4].copy_from_slice(&(paragraphs as u16).to_le_bytes());
        assert!(AtrImage::from_bytes(unaligned).is_err());
    }

    #[test]
    fn maps_128_byte_sectors_and_rejects_invalid_numbers() {
        let mut bytes = atr_bytes(128, 4);
        bytes[16] = 1;
        bytes[16 + 3 * 128] = 4;
        let image = AtrImage::from_bytes(bytes).unwrap();

        assert_eq!(image.sector_size(), 128);
        assert_eq!(image.sector_count(), 4);
        assert_eq!(image.read_sector(1).unwrap()[0], 1);
        assert_eq!(image.read_sector(4).unwrap()[0], 4);
        assert!(image.read_sector(0).is_err());
        assert!(image.read_sector(5).is_err());
    }

    #[test]
    fn keeps_first_three_sectors_short_on_256_byte_images() {
        let mut bytes = atr_bytes(256, 5);
        bytes[16 + 2 * 128] = 3;
        bytes[16 + 3 * 128] = 4;
        bytes[16 + 3 * 128 + 256] = 5;
        let image = AtrImage::from_bytes(bytes).unwrap();

        assert_eq!(image.sector_size(), 256);
        assert_eq!(image.sector_count(), 5);
        assert_eq!(image.sector_len(1).unwrap(), 128);
        assert_eq!(image.sector_len(3).unwrap(), 128);
        assert_eq!(image.sector_len(4).unwrap(), 256);
        assert_eq!(image.read_sector(3).unwrap()[0], 3);
        assert_eq!(image.read_sector(4).unwrap()[0], 4);
        assert_eq!(image.read_sector(5).unwrap()[0], 5);
    }

    #[test]
    fn writes_exact_sectors_and_tracks_real_changes() {
        let bytes = atr_bytes(256, 4);
        let original = bytes.clone();
        let mut image = AtrImage::from_bytes(bytes).unwrap();

        image.write_sector(1, &[0; 128]).unwrap();
        assert!(!image.is_dirty());
        assert!(image.write_sector(1, &[1; 256]).is_err());

        image.write_sector(4, &[0xA5; 256]).unwrap();
        assert!(image.is_dirty());
        assert_eq!(image.dirty_sectors(), vec![4]);
        assert_eq!(image.read_sector(4).unwrap(), &[0xA5; 256]);
        assert_eq!(&original[0..16], &image.as_bytes()[0..16]);
        assert_eq!(image.original_bytes(), original);

        image.write_sector(4, &[0; 256]).unwrap();
        assert!(!image.is_dirty());
        image.write_sector(4, &[0xA5; 256]).unwrap();

        image.clear_dirty();
        assert!(!image.is_dirty());
        assert_eq!(image.original_bytes(), image.as_bytes());
        assert_eq!(image.into_bytes().len(), original.len());
    }
}
