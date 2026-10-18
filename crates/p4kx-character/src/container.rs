use std::io::Read;

use p4kx_foundation::ByteReader;
use ruzstd::decoding::StreamingDecoder;
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

use crate::ChfError;

/// The size of every CHF file.
pub const CHF_SIZE: usize = 4096;
/// The first two bytes of every CHF file, little-endian.
pub const CHF_MAGIC: u16 = 0x4242;
/// The largest decompressed body this crate reads or writes.
pub const MAX_BODY_LEN: usize = 64 * 1024;

const HEADER_LEN: usize = 16;
const MAX_FRAME_LEN: usize = CHF_SIZE - HEADER_LEN;
const MAX_WINDOW: u64 = 1024 * 1024;

/// The fixed-size outer layer of a CHF file: header, zstd frame and the bytes behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Container {
    pub(crate) flags: u16,
    pub(crate) frame: Vec<u8>,
    pub(crate) padding: Vec<u8>,
    pub(crate) body: Vec<u8>,
}

impl Container {
    /// Splits a CHF file into its header fields, its frame, its padding and its decompressed body.
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, ChfError> {
        if bytes.len() != CHF_SIZE {
            return Err(ChfError::WrongSize {
                expected: CHF_SIZE,
                actual: bytes.len(),
            });
        }
        let mut header = ByteReader::new(bytes);
        let magic = header.read_u16()?;
        if magic != CHF_MAGIC {
            return Err(ChfError::BadMagic {
                expected: CHF_MAGIC,
                found: magic,
            });
        }
        let flags = header.read_u16()?;
        let stored = header.read_u32()?;
        let frame_len = header.read_u32()?;
        let body_len = header.read_u32()?;
        let data = bytes.get(HEADER_LEN..).unwrap_or_default();
        let computed = crc32c::crc32c(data);
        if stored != computed {
            return Err(ChfError::CrcMismatch { stored, computed });
        }
        let frame = usize::try_from(frame_len)
            .ok()
            .and_then(|len| data.get(..len))
            .ok_or_else(|| ChfError::FrameTooLarge {
                size: frame_len.into(),
                max: MAX_FRAME_LEN,
            })?;
        let body = decompress(frame, body_len)?;
        let padding = data.get(frame.len()..).unwrap_or_default();
        Ok(Self {
            flags,
            frame: frame.to_vec(),
            padding: trim_zeros(padding).to_vec(),
            body,
        })
    }
}

/// Writes a CHF file, reusing `frame` when it still decompresses to `body`.
pub(crate) fn write(
    flags: u16,
    frame: &[u8],
    padding: &[u8],
    body: &[u8],
) -> Result<Vec<u8>, ChfError> {
    let body_len = u32::try_from(body.len())
        .ok()
        .filter(|len| *len as usize <= MAX_BODY_LEN)
        .ok_or(ChfError::BodyTooLarge {
            size: body.len() as u64,
            max: MAX_BODY_LEN,
        })?;
    let fresh;
    let frame =
        if !frame.is_empty() && decompress(frame, body_len).is_ok_and(|decoded| decoded == body) {
            frame
        } else {
            fresh = compress_to_vec(body, CompressionLevel::Fastest);
            fresh.as_slice()
        };
    let frame_len = u32::try_from(frame.len())
        .ok()
        .filter(|len| *len as usize <= MAX_FRAME_LEN)
        .ok_or(ChfError::FrameTooLarge {
            size: frame.len() as u64,
            max: MAX_FRAME_LEN,
        })?;
    let mut out = Vec::with_capacity(CHF_SIZE);
    out.extend_from_slice(&CHF_MAGIC.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&frame_len.to_le_bytes());
    out.extend_from_slice(&body_len.to_le_bytes());
    out.extend_from_slice(frame);
    let room = CHF_SIZE - out.len();
    out.extend_from_slice(padding.get(..room).unwrap_or(padding));
    out.resize(CHF_SIZE, 0);
    let crc = crc32c::crc32c(out.get(HEADER_LEN..).unwrap_or_default());
    if let Some(slot) = out.get_mut(4..8) {
        slot.copy_from_slice(&crc.to_le_bytes());
    }
    Ok(out)
}

/// Decompresses one zstd frame that must yield exactly `stored` bytes.
fn decompress(frame: &[u8], stored: u32) -> Result<Vec<u8>, ChfError> {
    let expected = stored as usize;
    if expected > MAX_BODY_LEN {
        return Err(ChfError::BodyTooLarge {
            size: stored.into(),
            max: MAX_BODY_LEN,
        });
    }
    let corrupt = |reason: &dyn std::fmt::Display| ChfError::CorruptFrame {
        reason: reason.to_string(),
    };
    let decoder = StreamingDecoder::new_with_max_window_size(frame, MAX_WINDOW)
        .map_err(|error| corrupt(&error))?;
    let mut body = Vec::with_capacity(expected);
    decoder
        .take(u64::from(stored) + 1)
        .read_to_end(&mut body)
        .map_err(|error| corrupt(&error))?;
    if body.len() != expected {
        return Err(ChfError::BodySizeMismatch {
            stored,
            actual: body.len(),
        });
    }
    Ok(body)
}

fn trim_zeros(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |at| at + 1);
    bytes.get(..end).unwrap_or_default()
}
