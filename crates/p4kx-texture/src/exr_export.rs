use std::io::{self, Write};

use flate2::Compression;
use flate2::write::ZlibEncoder;
use rayon::prelude::*;

use crate::{HdrImage, TextureError};

const MAGIC: [u8; 4] = [0x76, 0x2F, 0x31, 0x01];
const VERSION: [u8; 4] = [2, 0, 0, 0];
const ZIP_COMPRESSION: u8 = 3;
const LINES_PER_CHUNK: usize = 16;
const FLOAT: i32 = 2;
const CHANNELS: [&str; 3] = ["B", "G", "R"];
const BYTES_PER_VALUE: usize = 4;

/// Writes `image` to `out` as a single-part scanline OpenEXR file with 32-bit float `R`, `G`
/// and `B` channels, ZIP-compressed in chunks of 16 lines, chunks compressed in parallel.
///
/// # Errors
///
/// Returns [`TextureError::Write`] when `out` fails and [`TextureError::InvalidHeader`] when the
/// image is too large for OpenEXR.
pub fn write_exr<W: Write>(image: &HdrImage, mut out: W) -> Result<(), TextureError> {
    let too_large = || TextureError::InvalidHeader("the image is too large for OpenEXR");
    let width = usize::try_from(image.width()).map_err(|_| too_large())?;
    let max_x = i32::try_from(image.width()).map_err(|_| too_large())? - 1;
    let max_y = i32::try_from(image.height()).map_err(|_| too_large())? - 1;
    let line_len = width.checked_mul(3).ok_or_else(too_large)?;
    let chunks: Vec<Vec<u8>> = image
        .pixels()
        .par_chunks(line_len.saturating_mul(LINES_PER_CHUNK).max(1))
        .map(|lines| compress(lines, line_len))
        .collect::<io::Result<_>>()
        .map_err(TextureError::Write)?;
    let header = header(max_x, max_y);
    let mut offset = u64::try_from(MAGIC.len() + VERSION.len() + header.len())
        .ok()
        .and_then(|start| start.checked_add(8 * u64::try_from(chunks.len()).ok()?))
        .ok_or_else(too_large)?;
    let write = |out: &mut W, bytes: &[u8]| out.write_all(bytes).map_err(TextureError::Write);
    write(&mut out, &MAGIC)?;
    write(&mut out, &VERSION)?;
    write(&mut out, &header)?;
    for chunk in &chunks {
        write(&mut out, &offset.to_le_bytes())?;
        offset = offset
            .checked_add(8 + chunk.len() as u64)
            .ok_or_else(too_large)?;
    }
    for (chunk, first_line) in chunks.iter().zip((0_i32..).step_by(LINES_PER_CHUNK)) {
        let len = i32::try_from(chunk.len()).map_err(|_| too_large())?;
        write(&mut out, &first_line.to_le_bytes())?;
        write(&mut out, &len.to_le_bytes())?;
        write(&mut out, chunk)?;
    }
    out.flush().map_err(TextureError::Write)
}

fn header(max_x: i32, max_y: i32) -> Vec<u8> {
    let mut channels = Vec::new();
    for name in CHANNELS {
        channels.extend_from_slice(name.as_bytes());
        channels.push(0);
        channels.extend_from_slice(&FLOAT.to_le_bytes());
        channels.extend_from_slice(&[0; 4]);
        channels.extend_from_slice(&1_i32.to_le_bytes());
        channels.extend_from_slice(&1_i32.to_le_bytes());
    }
    channels.push(0);
    let window: Vec<u8> = [0, 0, max_x, max_y]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let mut header = Vec::new();
    let mut attribute = |name: &str, kind: &str, value: &[u8]| {
        header.extend_from_slice(name.as_bytes());
        header.push(0);
        header.extend_from_slice(kind.as_bytes());
        header.push(0);
        header.extend_from_slice(&u32::try_from(value.len()).unwrap_or_default().to_le_bytes());
        header.extend_from_slice(value);
    };
    attribute("channels", "chlist", &channels);
    attribute("compression", "compression", &[ZIP_COMPRESSION]);
    attribute("dataWindow", "box2i", &window);
    attribute("displayWindow", "box2i", &window);
    attribute("lineOrder", "lineOrder", &[0]);
    attribute("pixelAspectRatio", "float", &1.0_f32.to_le_bytes());
    attribute("screenWindowCenter", "v2f", &[0; 8]);
    attribute("screenWindowWidth", "float", &1.0_f32.to_le_bytes());
    header.push(0);
    header
}

/// Lays out `lines` of RGB pixels channel by channel per line, splits and predicts the bytes as
/// OpenEXR's ZIP compression does and deflates them; data that does not shrink stays raw.
fn compress(lines: &[f32], line_len: usize) -> io::Result<Vec<u8>> {
    let mut raw = Vec::with_capacity(lines.len() * BYTES_PER_VALUE);
    for line in lines.chunks(line_len.max(1)) {
        let (pixels, _) = line.as_chunks::<3>();
        for channel in [2, 1, 0] {
            for pixel in pixels {
                let value = pixel.get(channel).copied().unwrap_or_default();
                raw.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    let (even, odd): (Vec<u8>, Vec<u8>) = (
        raw.iter().step_by(2).copied().collect(),
        raw.iter().skip(1).step_by(2).copied().collect(),
    );
    let mut predicted = even;
    predicted.extend_from_slice(&odd);
    let mut previous = predicted.first().copied().unwrap_or_default();
    for byte in predicted.iter_mut().skip(1) {
        let current = *byte;
        *byte = current.wrapping_sub(previous).wrapping_add(128);
        previous = current;
    }
    let mut encoder = ZlibEncoder::new(Vec::with_capacity(raw.len() / 2), Compression::fast());
    encoder.write_all(&predicted)?;
    let compressed = encoder.finish()?;
    Ok(if compressed.len() < raw.len() {
        compressed
    } else {
        raw
    })
}
