use crate::GfxError;
use crate::bitmap::Lossless;
use crate::image::{RgbaImage, checked_pixels, inflate_exact};

const COLOR_MAPPED: u8 = 3;
const RGB15: u8 = 4;
const ARGB32: u8 = 5;

/// Decodes the pixels of a lossless bitmap to straight-alpha RGBA.
pub(crate) fn decode_lossless(bitmap: &Lossless<'_>) -> Result<RgbaImage, GfxError> {
    let width = usize::from(bitmap.width);
    let height = usize::from(bitmap.height);
    let pixels = checked_pixels(u64::from(bitmap.width), u64::from(bitmap.height))?;
    let entry_len = if bitmap.has_alpha { 4 } else { 3 };
    let (table_len, row_len) = match bitmap.format {
        COLOR_MAPPED => (
            usize::from(bitmap.color_table_len) * entry_len,
            width.next_multiple_of(4),
        ),
        RGB15 => (0, (width * 2).next_multiple_of(4)),
        ARGB32 => (0, width * 4),
        format => return Err(GfxError::LosslessFormat { format }),
    };
    let raw = inflate_exact(bitmap.data, table_len + row_len * height)?;
    let (table, rows) = raw.split_at(table_len);
    let mut out = Vec::with_capacity(pixels * 4);
    for row in rows.chunks_exact(row_len.max(1)).take(height) {
        match bitmap.format {
            COLOR_MAPPED => color_mapped_row(row, width, table, entry_len, &mut out),
            RGB15 => rgb15_row(row, width, &mut out),
            _ => argb32_row(row, width, bitmap.has_alpha, &mut out),
        }
    }
    let width = u32::from(bitmap.width);
    let height = u32::from(bitmap.height);
    Ok(RgbaImage::new(width, height, out))
}

fn color_mapped_row(row: &[u8], width: usize, table: &[u8], entry_len: usize, out: &mut Vec<u8>) {
    for &index in row.iter().take(width) {
        let start = usize::from(index) * entry_len;
        match table.get(start..start + entry_len) {
            Some(&[red, green, blue]) => out.extend_from_slice(&[red, green, blue, u8::MAX]),
            Some(&[red, green, blue, alpha]) => {
                out.extend_from_slice(&unmultiply(red, green, blue, alpha));
            }
            _ => out.extend_from_slice(&[0; 4]),
        }
    }
}

fn rgb15_row(row: &[u8], width: usize, out: &mut Vec<u8>) {
    for &pixel in row.as_chunks::<2>().0.iter().take(width) {
        let value = u16::from_be_bytes(pixel);
        out.extend_from_slice(&[
            expand5(value >> 10),
            expand5(value >> 5),
            expand5(value),
            u8::MAX,
        ]);
    }
}

fn argb32_row(row: &[u8], width: usize, premultiplied: bool, out: &mut Vec<u8>) {
    for &[alpha, red, green, blue] in row.as_chunks::<4>().0.iter().take(width) {
        if premultiplied {
            out.extend_from_slice(&unmultiply(red, green, blue, alpha));
        } else {
            out.extend_from_slice(&[red, green, blue, u8::MAX]);
        }
    }
}

fn expand5(value: u16) -> u8 {
    let value = u8::try_from(value & 0x1F).unwrap_or_default();
    (value << 3) | (value >> 2)
}

/// Returns the straight-alpha colour of a premultiplied one; a channel above the alpha is clamped
/// to it first, as Flash Player does.
pub(crate) fn unmultiply(red: u8, green: u8, blue: u8, alpha: u8) -> [u8; 4] {
    if alpha == 0 {
        return [0; 4];
    }
    let channel = |value: u8| {
        let straight = u16::from(value) * 255 / u16::from(alpha);
        u8::try_from(straight).unwrap_or(u8::MAX)
    };
    [channel(red), channel(green), channel(blue), alpha]
}
