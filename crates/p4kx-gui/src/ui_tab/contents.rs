// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::UiMovie;
use p4kx_extraction::gfx::{Assets, Bitmap, BitmapSource, Rect};
use p4kx_screens::Screen;

use crate::UiText;

/// The most rows the contents of one file or screen list.
pub const MAX_CONTENT_ROWS: usize = 20_000;

/// What a definition of a movie is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// A shape or morph shape of this size, e.g. `32 × 32 px`.
    Size(String),
    /// A bitmap held as lossless pixels of this size.
    Lossless(String),
    /// A bitmap held as an embedded image.
    Embedded,
    /// A bitmap loaded from a file, drawn at this size.
    External(String),
    /// A bitmap that is a part of another one.
    Part,
    /// A font with this name and this many glyphs.
    Font {
        /// The name of the font.
        name: String,
        /// The number of glyphs.
        glyphs: u32,
    },
    /// A sprite with this many frames.
    Sprite {
        /// The number of frames.
        frames: u32,
    },
}

/// The details of one row of the contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowDetail {
    /// A text that is shown as it is: a path, a key or a widget type with its count.
    Plain(String),
    /// A definition of a movie with its identifier and the name it is exported under.
    Definition {
        /// The identifier of the definition.
        id: u16,
        /// What it is.
        note: Note,
        /// The name it is exported under, if the movie names it.
        name: Option<String>,
    },
}

/// One definition of a movie, or one asset or widget type of a screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRow {
    /// The label that names the kind.
    pub kind: UiText,
    /// The details.
    pub detail: RowDetail,
}

/// The value of a fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    /// A number of things, shown with the digit grouping of the language.
    Count(usize),
    /// A text that is shown as it is.
    Text(String),
}

/// What a UI file or a screen is made of: a few facts and the list of its parts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiContents {
    facts: Vec<(UiText, Fact)>,
    rows: Vec<ContentRow>,
    omitted: usize,
}

impl UiContents {
    /// Returns the facts: label and value.
    #[must_use]
    pub fn facts(&self) -> &[(UiText, Fact)] {
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

    fn push(&mut self, kind: UiText, detail: RowDetail) {
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
            let detail = definition(assets, shape.id, Note::Size(pixels(shape.bounds)));
            contents.push(UiText::KindShape, detail);
        }
        for shape in &assets.morph_shapes {
            let detail = definition(assets, shape.id, Note::Size(pixels(shape.start_bounds)));
            contents.push(UiText::KindMorphShape, detail);
        }
        for bitmap in &assets.bitmaps {
            let detail = definition(assets, bitmap.id, bitmap_note(bitmap));
            contents.push(UiText::KindBitmap, detail);
        }
        for font in &assets.fonts {
            let note = Note::Font {
                name: font.name.clone(),
                glyphs: font.glyph_count,
            };
            contents.push(UiText::KindFont, definition(assets, font.id, note));
        }
        for sprite in &assets.sprites {
            let note = Note::Sprite {
                frames: u32::from(sprite.frame_count),
            };
            contents.push(UiText::KindSprite, definition(assets, sprite.id, note));
        }
        for import in &assets.imports {
            let names: Vec<&str> = import
                .assets
                .iter()
                .map(|asset| asset.name.as_str())
                .collect();
            contents.push(
                UiText::KindImport,
                RowDetail::Plain(format!("{}: {}", import.url, names.join(", "))),
            );
        }
        contents
    }

    pub(super) fn of_screen(screen: &Screen) -> Self {
        let assets = &screen.assets;
        let mut contents = Self {
            facts: vec![
                (UiText::FactEntity, Fact::Text(screen.entity.clone())),
                (UiText::FactPort, Fact::Text(screen.port.clone())),
                (
                    UiText::FactSize,
                    Fact::Text(format!("{} × {} px", screen.width, screen.height)),
                ),
                (UiText::FactCanvases, Fact::Count(assets.canvases.len())),
                (UiText::FactTextures, Fact::Count(assets.textures.len())),
                (UiText::FactVectors, Fact::Count(assets.vectors.len())),
                (UiText::FactMovies, Fact::Count(assets.movies.len())),
                (
                    UiText::FactKeys,
                    Fact::Count(assets.localisation_keys.len()),
                ),
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
                contents.push(kind, RowDetail::Plain(path.clone()));
            }
        }
        for element in &screen.elements {
            let kind = if element.rendered {
                UiText::KindElement
            } else {
                UiText::KindElementUndrawn
            };
            contents.push(
                kind,
                RowDetail::Plain(format!("{} {}", element.element, element.count)),
            );
        }
        contents
    }
}

fn movie_facts(movie: &UiMovie<'_>, assets: &Assets<'_>) -> Vec<(UiText, Fact)> {
    let header = movie.movie().header();
    let tags: u32 = assets.tag_counts.iter().map(|tag| tag.count).sum();
    let count = Fact::Count;
    vec![
        (
            UiText::FactFormat,
            Fact::Text(format!("{} {}", header.signature.as_str(), header.version)),
        ),
        (
            UiText::FactStage,
            Fact::Text(format!(
                "{} × {} px",
                header.frame_size.width() / 20,
                header.frame_size.height() / 20
            )),
        ),
        (
            UiText::FactFrames,
            Fact::Text(format!(
                "{} @ {} fps",
                header.frame_count, header.frame_rate
            )),
        ),
        (
            UiText::FactTags,
            count(usize::try_from(tags).unwrap_or(usize::MAX)),
        ),
        (
            UiText::FactShapes,
            count(assets.shapes.len() + assets.morph_shapes.len()),
        ),
        (UiText::FactBitmaps, count(assets.bitmaps.len())),
        (UiText::FactFonts, count(assets.fonts.len())),
        (UiText::FactSprites, count(assets.sprites.len())),
    ]
}

fn definition(assets: &Assets<'_>, id: u16, note: Note) -> RowDetail {
    RowDetail::Definition {
        id,
        note,
        name: assets.export_name(id).map(str::to_owned),
    }
}

fn pixels(bounds: Rect) -> String {
    format!("{} × {} px", bounds.width() / 20, bounds.height() / 20)
}

fn bitmap_note(bitmap: &Bitmap<'_>) -> Note {
    match &bitmap.source {
        BitmapSource::Lossless(lossless) => {
            Note::Lossless(format!("{} × {} px", lossless.width, lossless.height))
        }
        BitmapSource::Jpeg(_) => Note::Embedded,
        BitmapSource::External(image) => Note::External(format!(
            "{} × {} px",
            image.target_width, image.target_height
        )),
        BitmapSource::SubImage(_) => Note::Part,
    }
}
