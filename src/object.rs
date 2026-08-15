use crate::{Memory, RUNAD};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtariLoadReport {
    pub segments: Vec<AtariLoadSegment>,
    pub run_address: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtariLoadSegment {
    pub start: u16,
    pub end: u16,
    pub len: usize,
}

pub fn load_atari_object_into_memory(
    memory: &mut Memory,
    bytes: &[u8],
) -> Result<AtariLoadReport, String> {
    let mut offset = 0usize;
    let mut segments = Vec::new();
    let mut run_address = None;

    while offset < bytes.len() {
        if bytes.len().saturating_sub(offset) < 4 {
            return Err(format!(
                "truncated Atari load segment header at file offset {offset}"
            ));
        }

        if bytes[offset] == 0xFF && bytes[offset + 1] == 0xFF {
            offset += 2;
            if offset == bytes.len() {
                break;
            }
        }

        if bytes.len().saturating_sub(offset) < 4 {
            return Err(format!(
                "truncated Atari load segment address pair at file offset {offset}"
            ));
        }

        let start = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let end = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
        offset += 4;
        if end < start {
            return Err(format!(
                "invalid Atari load segment ${start:04X}-${end:04X}"
            ));
        }

        let len = usize::from(end.wrapping_sub(start)) + 1;
        let data_end = offset
            .checked_add(len)
            .ok_or_else(|| "Atari load segment length overflowed".to_string())?;
        if data_end > bytes.len() {
            return Err(format!(
                "segment ${start:04X}-${end:04X} needs {len} byte(s), only {} remain",
                bytes.len().saturating_sub(offset)
            ));
        }

        memory.map(start, &bytes[offset..data_end])?;
        if start <= RUNAD && end >= RUNAD.wrapping_add(1) {
            let run_offset = offset + usize::from(RUNAD.wrapping_sub(start));
            run_address = Some(u16::from_le_bytes([
                bytes[run_offset],
                bytes[run_offset + 1],
            ]));
        }
        segments.push(AtariLoadSegment { start, end, len });
        offset = data_end;
    }

    Ok(AtariLoadReport {
        segments,
        run_address,
    })
}
