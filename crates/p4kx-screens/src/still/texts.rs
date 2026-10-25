// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::DataForge;
use p4kx_extraction::{
    Archive, GlobalIni, Language, LocalizationEntry, UiMovie, decode_line_breaks,
};
use p4kx_gfx::Font;

use crate::compose::ScreenComposer;

/// The text a localisation key resolved to, with its line breaks decoded.
pub(super) fn decoded(text: &str) -> String {
    decode_line_breaks(text).into_owned()
}

/// The fonts and the `global.ini` the texts of screens are drawn with, read from an Archive once
/// and used for any number of [`Still`](super::Still)s.
#[derive(Debug)]
pub struct ScreenTexts {
    fonts: Option<Vec<Font>>,
    entry: Option<LocalizationEntry>,
    warnings: Vec<String>,
}

impl ScreenTexts {
    /// Reads the fonts the Record `uielements/buildingblocks` of `forge` names and, when
    /// `language` is given, the `global.ini` of that language from `archive`.
    ///
    /// A source that cannot be read is left out and named in [`ScreenTexts::warnings`]: without
    /// the fonts text widgets are not drawn, without the texts localisation keys are drawn as
    /// they are.
    #[must_use]
    pub fn load(archive: &Archive, forge: &DataForge<'_>, language: Option<Language>) -> Self {
        let mut warnings = Vec::new();
        let font_file = ScreenComposer::new(forge).font_file();
        let fonts = read_fonts(archive, &font_file)
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

    pub(super) fn fonts(&self) -> Option<&[Font]> {
        self.fonts.as_deref()
    }

    pub(super) fn parsed(&self, warnings: &mut Vec<String>) -> Option<GlobalIni<'_>> {
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
