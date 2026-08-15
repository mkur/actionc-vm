use crate::RAM_SIZE;

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

pub(crate) fn mapped_end(base: u16, size: usize) -> Result<u16, String> {
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
