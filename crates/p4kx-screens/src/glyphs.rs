// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::{Affine, ColorTransform, Origin, Rgba, SceneItem, SceneShape};

use crate::paint::{Painter, TWIPS};

/// The font, the glyph (`usize::MAX` for the empty box of a missing glyph) and the colour a
/// glyph shape was converted for.
pub(crate) type GlyphKey = (usize, usize, [u8; 4]);

pub(crate) const MISSING: usize = usize::MAX;

pub(crate) const fn key(font: usize, glyph: usize, colour: Rgba) -> GlyphKey {
    (
        font,
        glyph,
        [colour.red, colour.green, colour.blue, colour.alpha],
    )
}

impl Painter {
    /// Places the glyph shape of `key` with its origin at `at` pixels, scaled by `scale` twips
    /// per unit of the shape; `make` converts the shape the first time the key is placed.
    pub(crate) fn glyph(
        &mut self,
        key: GlyphKey,
        make: impl FnOnce() -> Option<SceneShape>,
        (at, scale): ((f32, f32), f32),
    ) {
        if self.is_full() {
            return;
        }
        let shape = if let Some(&known) = self.glyphs.get(&key) {
            known
        } else {
            let converted = make().map(|shape| {
                self.shapes.push(shape);
                self.shapes.len() - 1
            });
            self.glyphs.insert(key, converted);
            converted
        };
        let Some(shape) = shape else {
            return;
        };
        let id = self.shapes.get(shape).map_or(0, |shape| shape.id);
        self.items.push(SceneItem {
            shape,
            transform: Affine {
                a: scale,
                b: 0.0,
                c: 0.0,
                d: scale,
                tx: at.0 * TWIPS,
                ty: at.1 * TWIPS,
            },
            color_transform: ColorTransform::IDENTITY,
            mask: None,
            origin: Origin {
                id,
                depth: 0,
                sprite: None,
            },
        });
    }
}
