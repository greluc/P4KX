// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use tiny_skia::{Path, Rect, Transform};

use crate::image::MAX_BITMAP_PIXELS;
use crate::scene::StrokeStyle;

/// How many times the area of the canvas the fills, strokes and masks of one rasterisation may
/// cover in total, each counted by its bounds clipped to the canvas.
pub const MAX_COVERAGE_FACTOR: u64 = 64;

/// The most bytes an SVG document may have.
pub const MAX_SVG_BYTES: u64 = 1 << 30;

/// The limits of one rasterisation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RasterLimits {
    pub(crate) coverage_factor: u64,
    pub(crate) mask_bytes: u64,
    pub(crate) bitmap_pixels: u64,
    pub(crate) svg_bytes: u64,
}

impl RasterLimits {
    pub(crate) const DEFAULT: Self = Self {
        coverage_factor: MAX_COVERAGE_FACTOR,
        mask_bytes: 1 << 28,
        bitmap_pixels: 4 * MAX_BITMAP_PIXELS,
        svg_bytes: MAX_SVG_BYTES,
    };
}

/// The pixels a rasterisation may still cover.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Coverage {
    remaining: u64,
}

impl Coverage {
    /// Starts a budget of `factor` times the pixels of a canvas of `width` x `height`.
    pub(crate) fn new(width: u32, height: u32, factor: u64) -> Self {
        Self {
            remaining: (u64::from(width) * u64::from(height)).saturating_mul(factor),
        }
    }

    /// Takes `pixels` from the budget, `false` and nothing taken when they do not fit.
    pub(crate) const fn charge(&mut self, pixels: u64) -> bool {
        match self.remaining.checked_sub(pixels) {
            Some(rest) => {
                self.remaining = rest;
                true
            }
            None => false,
        }
    }
}

/// Returns the pixels the bounds of `outline` cover on a canvas of `width` x `height` after
/// `to_canvas`, clipped to the canvas, for a stroke grown by its reach and a pixel for the edge;
/// `None` when the bounds cannot be transformed.
pub(crate) fn covered_pixels(
    outline: &Path,
    to_canvas: Transform,
    stroke: Option<&StrokeStyle>,
    width: u32,
    height: u32,
) -> Option<u64> {
    let local = outline.bounds();
    let reach = stroke.map_or(0.0, |style| {
        style.width.max(0.0) * style.miter_limit.max(1.0) / 2.0
    });
    let grown = Rect::from_ltrb(
        local.left() - reach,
        local.top() - reach,
        local.right() + reach,
        local.bottom() + reach,
    )?;
    let device = grown.transform(to_canvas)?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "canvas sides are at most MAX_CANVAS_SIDE"
    )]
    let (canvas_width, canvas_height) = (width as f32, height as f32);
    let left = (device.left() - 1.0).max(0.0);
    let top = (device.top() - 1.0).max(0.0);
    let right = (device.right() + 1.0).min(canvas_width);
    let bottom = (device.bottom() + 1.0).min(canvas_height);
    if right <= left || bottom <= top {
        return Some(0);
    }
    Some(pixels_of(right - left) * pixels_of(bottom - top))
}

const fn pixels_of(extent: f32) -> u64 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the extent is positive and at most the canvas side"
    )]
    let pixels = extent.ceil().max(1.0) as u64;
    pixels
}
