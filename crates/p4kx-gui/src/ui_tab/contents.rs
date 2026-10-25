// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::UiMovie;
use p4kx_extraction::gfx::{Assets, Bitmap, BitmapSource, Rect};
use p4kx_screens::Screen;

use crate::UiText;

/// The most rows the contents of one file or screen list.
pub const MAX_CONTENT_ROWS: usize = 20_000;

/// One definition of a movie, or one asset or widget type of a screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRow {
    /// The label that names the kind.
    pub kind: UiText,
    /// The details.
    pub detail: String,
}

/// What a UI file or a screen is made of: a few facts and the list of its parts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiContents {
    facts: Vec<(UiText, String)>,
    rows: Vec<ContentRow>,
    omitted: usize,
}

impl UiContents {
    /// Returns the facts: label and value.
    #[must_use]
    pub fn facts(&self) -> &[(UiText, String)] {
        &self.facts
    }

    /// Returns the parts, in the order `p4kx ui assets` lists them.
    #[must_use]
    pub fn rows(&self) -> &[ContentRow] {
        &self.rows
    }

    /// Returns how many parts the list leaves out beyond [`MAX_CONTENT_ROWS`].
    #[must_use]
    pub const fn omitted(&self) -> usize {
        self.omitted
    }

    fn push(&mut self, kind: UiText, detail: String) {
        if self.rows.len() < MAX_CONTENT_ROWS {
            self.rows.push(ContentRow { kind, detail });
        } else {
            self.omitted += 1;
        }
    }

    pub(super) fn of_movie(movie: &UiMovie<'_>, assets: &Assets<'_>) -> Self {
        let mut contents = Self {
            facts: movie_facts(movie, assets),
            ..Self::default()
        };
        for shape in &assets.shapes {
            let detail = definition(assets, shape.id, &pixels(shape.bounds));
            contents.push(UiText::KindShape, detail);
        }
        for shape in &assets.morph_shapes {
            let detail = definition(assets, shape.id, &pixels(shape.start_bounds));
            contents.push(UiText::KindMorphShape, detail);
        }
        for bitmap in &assets.bitmaps {
            let detail = definition(assets, bitmap.id, &bitmap_kind(bitmap));
            contents.push(UiText::KindBitmap, detail);
        }
        for font in &assets.fonts {
            let detail = definition(
                assets,
                font.id,
                &format!("{}, {} glyphs", font.name, font.glyph_count),
            );
            contents.push(UiText::KindFont, detail);
        }
        for sprite in &assets.sprites {
            let detail = definition(assets, sprite.id, &format!("{} frames", sprite.frame_count));
            contents.push(UiText::KindSprite, detail);
        }
        for import in &assets.imports {
            let names: Vec<&str> = import.assets.iter().map(|asset| asset.name.as_str()).collect();
            contents.push(
                UiText::KindImport,
                format!("{}: {}", import.url, names.join(", ")),
            );
        }
        contents
    }

    pub(super) fn of_screen(screen: &Screen) -> Self {
        let assets = &screen.assets;
        let mut contents = Self {
            facts: vec![
                (UiText::FactEntity, screen.entity.clone()),
                (UiText::FactPort, screen.port.clone()),
                (
                    UiText::FactSize,
                    format!("{} × {} px", screen.width, screen.height),
                ),
                (UiText::FactCanvases, assets.canvases.len().to_string()),
                (UiText::FactTextures, assets.textures.len().to_string()),
                (UiText::FactVectors, assets.vectors.len().to_string()),
                (UiText::FactMovies, assets.movies.len().to_string()),
                (UiText::FactKeys, assets.localisation_keys.len().to_string()),
            ],
            ..Self::default()
        };
        for (kind, list) in [
            (UiText::KindCanvas, &assets.canvases),
            (UiText::KindTexture, &assets.textures),
            (UiText::KindVector, &assets.vectors),
            (UiText::KindMovie, &assets.movies),
            (UiText::KindKey, &assets.localisation_keys),
        ] {
            for path in list {
                contents.push(kind, path.clone());
            }
        }
        for element in &screen.elements {
            let kind = if element.rendered {
                UiText::KindElement
            } else {
                UiText::KindElementUndrawn
            };
            contents.push(kind, format!("{} {}", element.element, element.count));
        }
        contents
    }
}

fn movie_facts(movie: &UiMovie<'_>, assets: &Assets<'_>) -> Vec<(UiText, String)> {
    let header = movie.movie().header();
    let tags: u32 = assets.tag_counts.iter().map(|tag| tag.count).sum();
    vec![
        (
            UiText::FactFormat,
            format!("{} {}", header.signature.as_str(), header.version),
        ),
        (
            UiText::FactStage,
            format!(
                "{} × {} px",
                header.frame_size.width() / 20,
                header.frame_size.height() / 20
            ),
        ),
        (
            UiText::FactFrames,
            format!("{} @ {} fps", header.frame_count, header.frame_rate),
        ),
        (UiText::FactTags, tags.to_string()),
        (
            UiText::FactShapes,
            (assets.shapes.len() + assets.morph_shapes.len()).to_string(),
        ),
        (UiText::FactBitmaps, assets.bitmaps.len().to_string()),
        (UiText::FactFonts, assets.fonts.len().to_string()),
        (UiText::FactSprites, assets.sprites.len().to_string()),
    ]
}

fn definition(assets: &Assets<'_>, id: u16, what: &str) -> String {
    assets.export_name(id).map_or_else(
        || format!("#{id}  {what}"),
        |name| format!("#{id}  {what}  {name}"),
    )
}

fn pixels(bounds: Rect) -> String {
    format!("{} × {} px", bounds.width() / 20, bounds.height() / 20)
}

fn bitmap_kind(bitmap: &Bitmap<'_>) -> String {
    match &bitmap.source {
        BitmapSource::Lossless(lossless) => {
            format!("lossless, {} × {} px", lossless.width, lossless.height)
        }
        BitmapSource::Jpeg(_) => "embedded image".to_owned(),
        BitmapSource::External(image) => format!(
            "external image, {} × {} px",
            image.target_width, image.target_height
        ),
        BitmapSource::SubImage(_) => "part of another bitmap".to_owned(),
    }
}
