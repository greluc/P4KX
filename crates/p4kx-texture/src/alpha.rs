// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use rayon::prelude::*;

use crate::{DxgiFormat, RgbaImage, TextureError, TextureFormat};

const RGBA: usize = 4;
const MIN_PIXELS_PER_TASK: usize = 16 * 1024;
const A8_UNORM: TextureFormat = TextureFormat::Dxgi(DxgiFormat(65));

/// Writes the decoded alpha or smoothness tail `alpha`, stored in `format`, into the alpha
/// channel of `image`; colour stays as it is.
///
/// The value comes from the tail's alpha channel when `format` is alpha-only (`A8_UNORM`) and
/// from its red channel otherwise, where single-channel formats such as BC4 keep it.
///
/// # Errors
///
/// Returns [`TextureError::AlphaTailSize`] when the two images differ in size; `image` is then
/// unchanged.
pub fn merge_alpha(
    image: &mut RgbaImage,
    alpha: &RgbaImage,
    format: TextureFormat,
) -> Result<(), TextureError> {
    if (alpha.width(), alpha.height()) != (image.width(), image.height()) {
        return Err(TextureError::AlphaTailSize {
            width: alpha.width(),
            height: alpha.height(),
            expected_width: image.width(),
            expected_height: image.height(),
        });
    }
    let from_alpha = format == A8_UNORM;
    let (targets, _) = image.pixels_mut().as_chunks_mut::<RGBA>();
    let (sources, _) = alpha.pixels().as_chunks::<RGBA>();
    targets
        .par_iter_mut()
        .zip(sources.par_iter())
        .with_min_len(MIN_PIXELS_PER_TASK)
        .for_each(|(target, &[red, _, _, alpha])| {
            target[3] = if from_alpha { alpha } else { red };
        });
    Ok(())
}
