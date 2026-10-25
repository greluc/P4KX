// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::Font;

use crate::{Archive, GlobalIni, Language, LocalizationEntry, UiMovie};

/// The fonts and the `global.ini` the texts of screens are drawn with, read from an Archive once
/// and used for any number of drawings.
#[derive(Debug)]
pub struct ScreenTexts {
    fonts: Option<Vec<Font>>,
    entry: Option<LocalizationEntry>,
    warnings: Vec<String>,
}

impl ScreenTexts {
    /// Reads the fonts of the font movie at the Record path `font_file` and, when `language` is
    /// given, the `global.ini` of that language from `archive`.
    ///
    /// A source that cannot be read is left out and named in [`ScreenTexts::warnings`]: without
    /// the fonts text widgets are not drawn, without the texts localisation keys are drawn as
    /// they are.
    #[must_use]
    pub fn load(archive: &Archive, font_file: &str, language: Option<Language>) -> Self {
        let mut warnings = Vec::new();
        let fonts = read_fonts(archive, font_file)
            .map_err(|error| warnings.push(error))
            .ok();
        let entry = language.and_then(|language| {
            LocalizationEntry::read(archive, language)
                .map_err(|error| warnings.push(error.to_string()))
                .ok()
        });
        Self {
            fonts,
            entry,
            warnings,
        }
    }

    /// Returns why a font or text source was left out.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Returns whether the fonts were read, so that text widgets are drawn.
    #[must_use]
    pub const fn has_fonts(&self) -> bool {
        self.fonts.is_some()
    }

    /// Returns whether the texts of a language were read, so that localisation keys resolve.
    #[must_use]
    pub const fn has_texts(&self) -> bool {
        self.entry.is_some()
    }

    /// Returns the fonts, if they were read.
    #[must_use]
    pub fn fonts(&self) -> Option<&[Font]> {
        self.fonts.as_deref()
    }

    /// Parses the texts of the language, if they were read; a parse error is pushed to
    /// `warnings` and the texts are left out.
    pub fn parsed(&self, warnings: &mut Vec<String>) -> Option<GlobalIni<'_>> {
        self.entry.as_ref().and_then(|entry| {
            entry
                .parse()
                .map_err(|error| warnings.push(error.to_string()))
                .ok()
        })
    }
}

fn read_fonts(archive: &Archive, record_path: &str) -> Result<Vec<Font>, String> {
    let under_data = format!("Data/{record_path}");
    let movie = UiMovie::from_archive(archive, record_path)
        .or_else(|_| UiMovie::from_archive(archive, &under_data))
        .map_err(|error| format!("the font movie {record_path} of the screens: {error}"))?;
    Ok(movie.movie().assets().fonts)
}
