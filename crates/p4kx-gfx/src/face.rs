// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::font::Font;
use crate::geometry::Rgba;
use crate::scene::{LayerKind, Paint, SceneShape, ShapeLayer};
use crate::scene_text::{filled_glyph, tofu};
use crate::text_fonts::{EM, FontIndex, em};

const ASCENT_FALLBACK: f32 = 0.8;
const DESCENT_FALLBACK: f32 = 0.2;

/// The units of the em square of the empty box [`missing_glyph`] draws.
pub const MISSING_GLYPH_EM: f32 = EM;

/// A font of a movie prepared to lay out and draw text of a caller, who places glyphs itself.
///
/// All metrics are fractions of the em square, so a text of `size` pixels multiplies them with
/// `size`; outlines stay in the units of the font, and [`FontFace::units_per_em`] is the scale
/// that brings them to one em.
#[derive(Debug)]
pub struct FontFace<'a> {
    font: &'a Font,
    index: FontIndex,
}

impl<'a> FontFace<'a> {
    /// Indexes the characters and kerning pairs of `font`.
    #[must_use]
    pub fn new(font: &'a Font) -> Self {
        Self {
            font,
            index: FontIndex::new(font),
        }
    }

    /// Returns the font.
    #[must_use]
    pub const fn font(&self) -> &'a Font {
        self.font
    }

    /// Returns whether the font has glyph outlines to draw; compacted fonts and fonts whose
    /// shapes the exporter stripped have none.
    #[must_use]
    pub fn is_drawable(&self) -> bool {
        self.font
            .glyphs
            .iter()
            .any(|glyph| !glyph.records.is_empty())
    }

    /// Returns the units of the em square of the outlines.
    #[must_use]
    pub fn units_per_em(&self) -> f32 {
        em(self.font)
    }

    /// Returns the index of the glyph that draws `ch`.
    #[must_use]
    pub fn glyph(&self, ch: char) -> Option<usize> {
        self.index.glyph(ch)
    }

    /// Returns the horizontal advance of a glyph in ems, `None` for a font without layout data.
    #[must_use]
    pub fn advance(&self, glyph: usize) -> Option<f32> {
        let advance = self.font.glyphs.get(glyph)?.advance?;
        Some(f32::from(advance) / self.units_per_em())
    }

    /// Returns the change of the advance between two characters in ems.
    #[must_use]
    pub fn kerning(&self, left: char, right: char) -> f32 {
        f32::from(self.index.kerning(left, right)) / self.units_per_em()
    }

    /// Returns the ascent and the descent in ems.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "font metrics fit an f32 closely enough"
    )]
    pub fn ascent_descent(&self) -> (f32, f32) {
        self.font
            .layout
            .as_ref()
            .map_or((ASCENT_FALLBACK, DESCENT_FALLBACK), |layout| {
                let units = self.units_per_em();
                (
                    layout.ascent as f32 / units,
                    (layout.descent as f32).abs() / units,
                )
            })
    }

    /// Returns the outline of a glyph filled with `color`, in the units of the font with the
    /// baseline at zero, or `None` for a glyph the font does not have or that has no outline.
    #[must_use]
    pub fn outline(&self, glyph: usize, color: Rgba) -> Option<SceneShape> {
        let records = &self.font.glyphs.get(glyph)?.records;
        if records.is_empty() {
            return None;
        }
        Some(filled_glyph(self.font.id, color, records).0)
    }
}

/// Returns the empty box drawn for a character without a glyph, filled with `color`, in the
/// units of [`MISSING_GLYPH_EM`] with the baseline at zero.
#[must_use]
pub fn missing_glyph(color: Rgba) -> SceneShape {
    SceneShape {
        id: u16::MAX,
        layers: vec![ShapeLayer {
            path: tofu(),
            paint: Paint::Solid(color),
            kind: LayerKind::Fill,
        }],
    }
}
