use std::ops::Range;

use crate::AudioError;

const HEADER_LEN: usize = 12;
const CHUNK_HEADER_LEN: usize = 8;

/// The byte order of a WEM: little-endian `RIFF` or big-endian `RIFX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteOrder {
    /// A `RIFF` file.
    Little,
    /// A `RIFX` file, as written for big-endian consoles.
    Big,
}

impl ByteOrder {
    pub(crate) fn u16_at(self, bytes: &[u8], offset: usize) -> Option<u16> {
        let field = *bytes.get(offset..)?.first_chunk::<2>()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(field),
            Self::Big => u16::from_be_bytes(field),
        })
    }

    pub(crate) fn u32_at(self, bytes: &[u8], offset: usize) -> Option<u32> {
        let field = *bytes.get(offset..)?.first_chunk::<4>()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(field),
            Self::Big => u32::from_be_bytes(field),
        })
    }
}

/// One chunk of a RIFF file: its id and the range of its body in the whole file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Chunk {
    pub(crate) id: [u8; 4],
    pub(crate) body: Range<usize>,
}

/// The chunks of a RIFF or RIFX WAVE file, in file order.
#[derive(Debug, Clone)]
pub(crate) struct Chunks<'a> {
    bytes: &'a [u8],
    order: ByteOrder,
    position: usize,
    end: usize,
}

impl<'a> Chunks<'a> {
    /// Checks the RIFF or RIFX WAVE header and returns its byte order and its chunks.
    pub(crate) fn of(bytes: &'a [u8]) -> Result<(ByteOrder, Self), AudioError> {
        let order = match bytes.first_chunk::<4>() {
            Some(b"RIFF") => ByteOrder::Little,
            Some(b"RIFX") => ByteOrder::Big,
            Some(b"PLUG") => return Err(AudioError::PluginMedia),
            Some(_) => return Err(AudioError::NotWem),
            None => return Err(truncated(bytes, HEADER_LEN as u64)),
        };
        let size = order
            .u32_at(bytes, 4)
            .ok_or_else(|| truncated(bytes, HEADER_LEN as u64))?;
        match bytes.get(8..HEADER_LEN) {
            Some(b"WAVE") => {}
            Some(_) => return Err(AudioError::NotWem),
            None => return Err(truncated(bytes, HEADER_LEN as u64)),
        }
        let end = u64::from(size) + CHUNK_HEADER_LEN as u64;
        let end = usize::try_from(end)
            .ok()
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| truncated(bytes, end))?;
        Ok((
            order,
            Self {
                bytes,
                order,
                position: HEADER_LEN,
                end,
            },
        ))
    }
}

impl Iterator for Chunks<'_> {
    type Item = Result<Chunk, AudioError>;

    fn next(&mut self) -> Option<Self::Item> {
        let header_end = self.position.checked_add(CHUNK_HEADER_LEN)?;
        if header_end > self.end {
            return None;
        }
        let id = *self.bytes.get(self.position..)?.first_chunk::<4>()?;
        let size = self.order.u32_at(self.bytes, self.position + 4)?;
        let body_end = header_end as u64 + u64::from(size);
        let Some(body_end) = usize::try_from(body_end)
            .ok()
            .filter(|&body_end| body_end <= self.end)
        else {
            self.position = self.end;
            return Some(Err(truncated(self.bytes, body_end)));
        };
        self.position = body_end
            .saturating_add(usize::from(size % 2 == 1))
            .min(self.end);
        Some(Ok(Chunk {
            id,
            body: header_end..body_end,
        }))
    }
}

const fn truncated(bytes: &[u8], needed: u64) -> AudioError {
    AudioError::Truncated {
        len: bytes.len() as u64,
        needed,
    }
}
