use std::io::Write;

use png::{BitDepth, ColorType, Compression, Encoder, EncodingError};

use crate::{RgbaImage, TextureError};

/// Writes `image` to `out` as an 8-bit RGBA PNG.
///
/// # Errors
///
/// Returns [`TextureError::Write`] when `out` fails and [`TextureError::EncodePng`] when the
/// image cannot be encoded.
pub fn write_png<W: Write>(image: &RgbaImage, out: W) -> Result<(), TextureError> {
    let mut encoder = Encoder::new(out, image.width(), image.height());
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::Fast);
    let mut writer = encoder.write_header().map_err(png_error)?;
    writer.write_image_data(image.pixels()).map_err(png_error)?;
    writer.finish().map_err(png_error)
}

fn png_error(error: EncodingError) -> TextureError {
    match error {
        EncodingError::IoError(source) => TextureError::Write(source),
        other => TextureError::EncodePng(other.to_string()),
    }
}
