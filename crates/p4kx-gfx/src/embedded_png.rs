use std::io::Cursor;

use png::{ColorType, Decoder, Limits, Reader, Transformations};

use crate::GfxError;
use crate::image::{MAX_BITMAP_PIXELS, RgbaImage, checked_pixels};

fn reader(data: &[u8]) -> Result<Reader<Cursor<&[u8]>>, GfxError> {
    let limits = Limits {
        bytes: usize::try_from(MAX_BITMAP_PIXELS * 8).unwrap_or(usize::MAX),
    };
    let mut decoder = Decoder::new_with_limits(Cursor::new(data), limits);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    decoder.read_info().map_err(|error| png_error(&error))
}

/// Returns the width and height an embedded PNG states.
pub(crate) fn png_size(data: &[u8]) -> Result<(u32, u32), GfxError> {
    let reader = reader(data)?;
    Ok((reader.info().width, reader.info().height))
}

/// Decodes an embedded PNG to straight-alpha RGBA.
pub(crate) fn decode_png(data: &[u8]) -> Result<RgbaImage, GfxError> {
    let mut reader = reader(data)?;
    let (width, height) = (reader.info().width, reader.info().height);
    let pixels = checked_pixels(u64::from(width), u64::from(height))?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| png_error(&"too large"))?
    ];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|error| png_error(&error))?;
    let samples = buffer.get(..frame.buffer_size()).unwrap_or_default();
    let mut rgba = Vec::with_capacity(pixels * 4);
    match frame.color_type {
        ColorType::Rgba => rgba.extend_from_slice(samples),
        ColorType::Rgb => {
            for pixel in samples.as_chunks::<3>().0 {
                rgba.extend_from_slice(pixel);
                rgba.push(u8::MAX);
            }
        }
        ColorType::GrayscaleAlpha => {
            for &[gray, alpha] in samples.as_chunks::<2>().0 {
                rgba.extend_from_slice(&[gray, gray, gray, alpha]);
            }
        }
        ColorType::Grayscale | ColorType::Indexed => {
            for &gray in samples {
                rgba.extend_from_slice(&[gray, gray, gray, u8::MAX]);
            }
        }
    }
    if rgba.len() != pixels * 4 {
        return Err(png_error(&"the pixel data does not match the size"));
    }
    Ok(RgbaImage::new(width, height, rgba))
}

fn png_error(error: &impl ToString) -> GfxError {
    GfxError::Png {
        message: error.to_string(),
    }
}
