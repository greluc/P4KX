// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cell::OnceCell;
use std::collections::HashMap;
use std::fmt;

use crate::font::{Font, FontFormat};
use crate::import::Import;

const EM: f32 = 1024.0;
const EM_FONT3: f32 = 20_480.0;
const ASCENT_FALLBACK: f32 = 0.8;
const DESCENT_FALLBACK: f32 = 0.2;

/// Looks a localisation key up, without its `@`.
pub type Localise<'a> = &'a dyn Fn(&str) -> Option<String>;

/// What text drawing may use beyond the movie itself.
///
/// No system font is ever loaded: a character whose font or glyph is missing is drawn as an empty
/// box and listed in the report.
#[derive(Clone, Copy, Default)]
pub struct TextOptions<'a> {
    /// Fonts of other movies, searched by name only after the fonts of the movie, because a font
    /// ID means something only inside its own movie.
    /// A font the movie imports is found by the name it imports it under.
    pub fonts: &'a [Font],
    /// Looks a localisation key up, without its `@`; `None` leaves the key as it is.
    pub localise: Option<Localise<'a>>,
}

impl fmt::Debug for TextOptions<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextOptions")
            .field("fonts", &self.fonts.len())
            .field("localise", &self.localise.is_some())
            .finish()
    }
}

/// The codes and kerning pairs of a font, indexed for lookup.
#[derive(Debug, Default)]
pub(crate) struct FontIndex {
    codes: HashMap<u16, usize>,
    kerning: HashMap<(u16, u16), i16>,
}

impl FontIndex {
    fn new(font: &Font) -> Self {
        let mut codes = HashMap::with_capacity(font.glyphs.len());
        for (at, glyph) in font.glyphs.iter().enumerate() {
            if let Some(code) = glyph.code {
                codes.entry(code).or_insert(at);
            }
        }
        let kerning = font.layout.as_ref().map_or_else(HashMap::new, |layout| {
            layout
                .kerning
                .iter()
                .map(|pair| ((pair.left, pair.right), pair.adjustment))
                .collect()
        });
        Self { codes, kerning }
    }

    /// Returns the index of the glyph that draws `ch`.
    pub(crate) fn glyph(&self, ch: char) -> Option<usize> {
        self.codes.get(&u16::try_from(u32::from(ch)).ok()?).copied()
    }

    /// Returns the change of the advance between two characters.
    pub(crate) fn kerning(&self, left: char, right: char) -> i16 {
        let pair = (
            u16::try_from(u32::from(left)),
            u16::try_from(u32::from(right)),
        );
        match pair {
            (Ok(left), Ok(right)) => self.kerning.get(&(left, right)).copied().unwrap_or(0),
            _ => 0,
        }
    }
}

/// The fonts a frame can draw with: those of the movie, then those of the options, which only a name finds.
pub(crate) struct Fonts<'b> {
    list: Vec<&'b Font>,
    usable: Vec<bool>,
    by_id: HashMap<u16, usize>,
    by_name: HashMap<String, usize>,
    indexes: Vec<OnceCell<FontIndex>>,
}

impl<'b> Fonts<'b> {
    pub(crate) fn new(movie: &'b [Font], extra: &'b [Font], imports: &[Import]) -> Self {
        let list: Vec<&Font> = movie.iter().chain(extra).collect();
        let mut by_id = HashMap::new();
        let mut by_name = HashMap::new();
        let mut by_export = HashMap::new();
        for (at, font) in list.iter().enumerate() {
            if at < movie.len() {
                by_id.entry(font.id).or_insert(at);
            }
            for name in std::iter::once(&font.name).chain(font.full_name.as_ref()) {
                if !name.is_empty() {
                    by_name.entry(name.to_ascii_lowercase()).or_insert(at);
                }
            }
            if let Some(name) = font.export_name.as_ref().filter(|name| !name.is_empty()) {
                by_export.entry(name.to_ascii_lowercase()).or_insert(at);
                by_name.entry(name.to_ascii_lowercase()).or_insert(at);
            }
        }
        for asset in imports.iter().flat_map(|import| &import.assets) {
            let name = asset.name.to_ascii_lowercase();
            if let Some(&at) = by_export.get(&name).or_else(|| by_name.get(&name)) {
                by_id.entry(asset.id).or_insert(at);
            }
        }
        let indexes = list.iter().map(|_| OnceCell::new()).collect();
        let usable = list
            .iter()
            .map(|font| font.glyphs.iter().any(|glyph| !glyph.records.is_empty()))
            .collect();
        Self {
            list,
            usable,
            by_id,
            by_name,
            indexes,
        }
    }

    pub(crate) fn by_id(&self, id: u16) -> Option<usize> {
        self.by_id.get(&id).copied()
    }

    pub(crate) fn by_name(&self, name: &str) -> Option<usize> {
        self.by_name.get(&name.to_ascii_lowercase()).copied()
    }

    /// Returns whether the font `at` has glyph outlines to draw.
    pub(crate) fn usable(&self, at: usize) -> bool {
        self.usable.get(at).copied().unwrap_or(false)
    }

    pub(crate) fn get(&self, at: usize) -> Option<&'b Font> {
        self.list.get(at).copied()
    }

    pub(crate) fn index(&self, at: usize) -> Option<&FontIndex> {
        let font = self.list.get(at)?;
        Some(self.indexes.get(at)?.get_or_init(|| FontIndex::new(font)))
    }
}

/// Returns the units of the em square of `font`.
pub(crate) fn em(font: &Font) -> f32 {
    if font.format == FontFormat::DefineFont3 {
        EM_FONT3
    } else {
        EM
    }
}

/// Returns the em square of the boxes drawn for missing glyphs.
pub(crate) const fn fallback_em() -> f32 {
    EM
}

/// Returns the ascent and descent of `font` at `height` twips.
pub(crate) fn vertical_metrics(font: &Font, height: f32) -> (f32, f32) {
    let scale = height / em(font);
    font.layout.as_ref().map_or(
        (height * ASCENT_FALLBACK, height * DESCENT_FALLBACK),
        |layout| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "font metrics fit an f32 closely enough"
            )]
            let (ascent, descent) = (layout.ascent as f32, layout.descent as f32);
            (ascent * scale, descent.abs() * scale)
        },
    )
}
