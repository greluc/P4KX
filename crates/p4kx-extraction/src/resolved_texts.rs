// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{Resolution, TextResolver};
use p4kx_localization::{Language, Lookup, OwnedTextSet, TextSet, decode_line_breaks};

#[derive(Debug, Clone, Copy)]
enum Source<'t, 'a> {
    Borrowed(&'t TextSet<'a>),
    Owned(&'t OwnedTextSet),
}

/// The texts of one language of a [`TextSet`] or an [`OwnedTextSet`], to write into Record XML
/// and Record JSON.
///
/// A key without a text in the language is missing; a placeholder key stays alone. The `\n`
/// escapes of the `global.ini` become line feeds.
#[derive(Debug, Clone, Copy)]
pub struct LanguageTexts<'t, 'a> {
    source: Source<'t, 'a>,
    language: Language,
}

impl<'t, 'a> LanguageTexts<'t, 'a> {
    /// Resolves in `language` with the files of `texts`.
    #[must_use]
    pub const fn new(texts: &'t TextSet<'a>, language: Language) -> Self {
        Self {
            source: Source::Borrowed(texts),
            language,
        }
    }

    /// Resolves in `language` with the texts of `texts`, which borrow nothing.
    #[must_use]
    pub const fn owned(texts: &'t OwnedTextSet, language: Language) -> Self {
        Self {
            source: Source::Owned(texts),
            language,
        }
    }

    /// Returns the language keys are resolved in.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }
}

impl TextResolver for LanguageTexts<'_, '_> {
    fn resolve<'r>(&'r self, key: &str) -> Resolution<'r> {
        let found = match self.source {
            Source::Borrowed(texts) => texts.lookup(key, self.language, None),
            Source::Owned(texts) => texts.lookup(key, self.language, None),
        };
        match found {
            Lookup::Found { text, .. } => Resolution::Text(decode_line_breaks(text)),
            Lookup::Placeholder => Resolution::Keep,
            _ => Resolution::Missing,
        }
    }
}
