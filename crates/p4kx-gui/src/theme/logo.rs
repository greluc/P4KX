//! The P4K-Extractor glyph of `docs/design/p4kx-glyph.svg`, rasterised from its geometry.
//!
//! The glyph is a ring open towards the lower right, a wedge reading through it and a data block
//! leaving it, on a 240 unit square. It appears only in the design system's logo colours.

use egui::{Color32, ColorImage, Context, Id, IconData, TextureHandle, TextureOptions};

use super::palette;

const VIEW_BOX: f32 = 240.0;
const SAMPLES: u16 = 4;
const RING_CENTRE: (f32, f32) = (110.0, 110.0);
const RING_RADIUS: f32 = 76.0;
const RING_HALF_WIDTH: f32 = 8.0;
const RING_GAP_FROM: (f32, f32) = (183.4, 129.7);
const RING_GAP_TO: (f32, f32) = (142.1, 178.9);
const WEDGE_ORIGIN: (f32, f32) = (20.0, 30.0);
const WEDGE_ANGLE_DEGREES: f32 = 41.0;
const WEDGE_LENGTH: f32 = 190.0;
const WEDGE_HALF_BASE: f32 = 26.0;
const BLOCK_MIN: (f32, f32) = (185.0, 171.0);
const BLOCK_SIZE: f32 = 30.0;
const ICON_SIZE: u32 = 128;
const MARK_SIZE: u32 = 96;
const MARK_TEXTURE: &str = "p4kx-brand-mark";

/// Returns the window and taskbar icon: the orange glyph on a black tile.
#[must_use]
pub fn window_icon() -> IconData {
    IconData {
        rgba: rgba(ICON_SIZE, palette::PRIMARY, Some(palette::BLACK)),
        width: ICON_SIZE,
        height: ICON_SIZE,
    }
}

/// Uploads the orange glyph on a transparent ground as the texture of the brand mark.
pub fn load_mark(ctx: &Context) {
    let size = usize::try_from(MARK_SIZE).unwrap_or(0);
    let pixels = rgba(MARK_SIZE, palette::PRIMARY, None);
    let image = ColorImage::from_rgba_unmultiplied([size, size], &pixels);
    let texture = ctx.load_texture(MARK_TEXTURE, image, TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(mark_id(), texture));
}

/// Returns the texture of the brand mark once [`load_mark`] uploaded it.
#[must_use]
pub fn mark(ctx: &Context) -> Option<TextureHandle> {
    ctx.data(|data| data.get_temp(mark_id()))
}

fn mark_id() -> Id {
    Id::new(MARK_TEXTURE)
}

/// Returns the glyph as unmultiplied RGBA rows of `size` pixels in `color` over `background`,
/// or over transparency.
#[must_use]
pub fn rgba(size: u32, color: Color32, background: Option<Color32>) -> Vec<u8> {
    let geometry = Geometry::new();
    let scale = VIEW_BOX / to_f32(size);
    let mut pixels = Vec::with_capacity(usize::try_from(size * size * 4).unwrap_or(0));
    for row in 0..size {
        for column in 0..size {
            let coverage = geometry.coverage(to_f32(column) * scale, to_f32(row) * scale, scale);
            pixels.extend_from_slice(&blend(color, background, coverage));
        }
    }
    pixels
}

fn blend(color: Color32, background: Option<Color32>, coverage: f32) -> [u8; 4] {
    let mix = |over: u8, under: u8| {
        let value = f32::from(under) + (f32::from(over) - f32::from(under)) * coverage;
        to_u8(value)
    };
    match background {
        Some(under) => [
            mix(color.r(), under.r()),
            mix(color.g(), under.g()),
            mix(color.b(), under.b()),
            255,
        ],
        None => [color.r(), color.g(), color.b(), to_u8(255.0 * coverage)],
    }
}

struct Geometry {
    gap_from: f32,
    gap_to: f32,
    sin: f32,
    cos: f32,
}

impl Geometry {
    fn new() -> Self {
        let angle = |(x, y): (f32, f32)| (y - RING_CENTRE.1).atan2(x - RING_CENTRE.0);
        let (sin, cos) = WEDGE_ANGLE_DEGREES.to_radians().sin_cos();
        Self {
            gap_from: angle(RING_GAP_FROM),
            gap_to: angle(RING_GAP_TO),
            sin,
            cos,
        }
    }

    fn coverage(&self, left: f32, top: f32, pixel: f32) -> f32 {
        let step = pixel / f32::from(SAMPLES);
        let mut inside = 0_u16;
        for sample_y in 0..SAMPLES {
            for sample_x in 0..SAMPLES {
                let x = (f32::from(sample_x) + 0.5).mul_add(step, left);
                let y = (f32::from(sample_y) + 0.5).mul_add(step, top);
                if self.contains(x, y) {
                    inside += 1;
                }
            }
        }
        f32::from(inside) / f32::from(SAMPLES * SAMPLES)
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        self.in_ring(x, y) || self.in_wedge(x, y) || in_block(x, y)
    }

    fn in_ring(&self, x: f32, y: f32) -> bool {
        let (dx, dy) = (x - RING_CENTRE.0, y - RING_CENTRE.1);
        let distance = dx.hypot(dy);
        let angle = dy.atan2(dx);
        (distance - RING_RADIUS).abs() <= RING_HALF_WIDTH
            && !(self.gap_from < angle && angle < self.gap_to)
    }

    fn in_wedge(&self, x: f32, y: f32) -> bool {
        let (dx, dy) = (x - WEDGE_ORIGIN.0, y - WEDGE_ORIGIN.1);
        let along = dx.mul_add(self.cos, dy * self.sin);
        let across = dy.mul_add(self.cos, -dx * self.sin);
        (0.0..=WEDGE_LENGTH).contains(&along)
            && across.abs() <= WEDGE_HALF_BASE * (1.0 - along / WEDGE_LENGTH)
    }
}

fn in_block(x: f32, y: f32) -> bool {
    (BLOCK_MIN.0..=BLOCK_MIN.0 + BLOCK_SIZE).contains(&x)
        && (BLOCK_MIN.1..=BLOCK_MIN.1 + BLOCK_SIZE).contains(&y)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "icon sizes are far below the f32 precision limit"
)]
const fn to_f32(value: u32) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to the u8 range first"
)]
fn to_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::{Geometry, ICON_SIZE, window_icon};

    #[test]
    fn the_glyph_covers_ring_wedge_and_block_but_not_the_ring_opening() {
        let geometry = Geometry::new();

        assert!(geometry.contains(110.0, 34.0), "top of the ring");
        assert!(geometry.contains(60.0, 70.0), "wedge");
        assert!(geometry.contains(200.0, 186.0), "data block");
        assert!(!geometry.contains(110.0, 110.0), "inside the ring");
        assert!(!geometry.contains(170.0, 160.0), "the opening");
    }

    #[test]
    fn the_window_icon_is_an_opaque_square() {
        let icon = window_icon();

        assert_eq!((icon.width, icon.height), (ICON_SIZE, ICON_SIZE), "square");
        assert!(
            icon.rgba.chunks_exact(4).all(|pixel| pixel.get(3) == Some(&255)),
            "black tile, no transparency"
        );
    }
}
