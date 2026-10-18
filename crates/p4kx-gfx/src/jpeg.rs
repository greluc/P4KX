use std::borrow::Cow;
use std::io::Cursor;

use png::{ColorType, Decoder, Limits, Transformations};
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use crate::GfxError;
use crate::bitmap::{EmbeddedFormat, Jpeg};
use crate::image::{MAX_BITMAP_PIXELS, RgbaImage, checked_pixels, inflate_exact};
use crate::lossless::unmultiply;

const END_THEN_START: [u8; 4] = [0xFF, 0xD9, 0xFF, 0xD8];
const MAX_SIDE: usize = 1 << 16;

/// Decodes an embedded JPEG or PNG to straight-alpha RGBA, with the JPEG tables of the movie for
/// a `DefineBits` image and the alpha channel of a `DefineBitsJPEG3` or `DefineBitsJPEG4` image,
/// whose colours are premultiplied.
pub(crate) fn decode_jpeg(jpeg: &Jpeg<'_>, tables: Option<&[u8]>) -> Result<RgbaImage, GfxError> {
    match jpeg.format() {
        EmbeddedFormat::Gif => Err(GfxError::Gif),
        EmbeddedFormat::Png => decode_png(jpeg.data),
        EmbeddedFormat::Jpeg => {
            let data = tables
                .filter(|_| jpeg.version == 1)
                .map_or(Cow::Borrowed(jpeg.data), |tables| {
                    Cow::Owned([tables, jpeg.data].concat())
                });
            let mut image = decode_jpeg_data(&without_end_then_start(&data))?;
            if let Some(alpha) = jpeg.alpha {
                apply_alpha(&mut image, alpha)?;
            }
            Ok(image)
        }
    }
}

/// Returns the JPEG data without the end-and-start marker pairs `FF D9 FF D8` that SWF files put
/// before an image and between the JPEG tables and an image.
fn without_end_then_start(data: &[u8]) -> Cow<'_, [u8]> {
    let Some(first) = data
        .windows(END_THEN_START.len())
        .position(|window| window == END_THEN_START)
    else {
        return Cow::Borrowed(data);
    };
    let mut cleaned = Vec::with_capacity(data.len());
    cleaned.extend_from_slice(data.get(..first).unwrap_or_default());
    for &byte in data.get(first..).unwrap_or_default() {
        cleaned.push(byte);
        if cleaned.ends_with(&END_THEN_START) {
            cleaned.truncate(cleaned.len() - END_THEN_START.len());
        }
    }
    Cow::Owned(cleaned)
}

fn decode_jpeg_data(data: &[u8]) -> Result<RgbaImage, GfxError> {
    let options = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGBA)
        .set_max_width(MAX_SIDE)
        .set_max_height(MAX_SIDE)
        .set_strict_mode(false);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(data), options);
    decoder
        .decode_headers()
        .map_err(|error| jpeg_error(&error))?;
    let (width, height) = decoder
        .info()
        .map(|info| (u32::from(info.width), u32::from(info.height)))
        .ok_or_else(|| GfxError::Jpeg {
            message: "the JPEG has no frame header".to_owned(),
        })?;
    let pixels = checked_pixels(u64::from(width), u64::from(height))?;
    let rgba = decoder.decode().map_err(|error| jpeg_error(&error))?;
    if rgba.len() != pixels * 4 {
        return Err(GfxError::Jpeg {
            message: format!("{} bytes decoded for {width} x {height} pixels", rgba.len()),
        });
    }
    Ok(RgbaImage::new(width, height, rgba))
}

fn jpeg_error(error: &impl ToString) -> GfxError {
    GfxError::Jpeg {
        message: error.to_string(),
    }
}

fn apply_alpha(image: &mut RgbaImage, alpha: &[u8]) -> Result<(), GfxError> {
    let pixels = checked_pixels(u64::from(image.width()), u64::from(image.height()))?;
    let alpha = inflate_exact(alpha, pixels)?;
    for (pixel, alpha) in image
        .pixels_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(alpha)
    {
        let [red, green, blue, _] = *pixel;
        *pixel = unmultiply(red, green, blue, alpha);
    }
    Ok(())
}

fn decode_png(data: &[u8]) -> Result<RgbaImage, GfxError> {
    let limits = Limits {
        bytes: usize::try_from(MAX_BITMAP_PIXELS * 8).unwrap_or(usize::MAX),
    };
    let mut decoder = Decoder::new_with_limits(Cursor::new(data), limits);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|error| png_error(&error))?;
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
