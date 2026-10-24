// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_extraction::{
    Archive, Language, LocalizationLoadError, LocalizationNote, LocalizationSet, Lookup, TextSet,
    decode_line_breaks, is_placeholder_key,
};

use crate::contract::LocalisedText;
use crate::{CatalogError, TextSummary};

/// The languages the catalog carries texts in, in the order its summary lists them.
pub(crate) const CATALOG_LANGUAGES: [Language; 2] = [Language::English, Language::German];

/// The `global.ini` Entry of every language in [`CATALOG_LANGUAGES`] the Archive holds.
pub(crate) struct TextSources {
    set: LocalizationSet,
}

impl TextSources {
    /// Reads the `global.ini` of every catalog language from `archive`.
    pub(crate) fn read(archive: &Archive) -> Result<Self, CatalogError> {
        let set =
            LocalizationSet::read(archive, &CATALOG_LANGUAGES).map_err(CatalogError::LoadTexts)?;
        Ok(Self { set })
    }
}

/// Resolves localisation keys in every catalog language and counts what resolves.
///
/// Every kind turns its keys into [`LocalisedText`]s through [`Texts::localised`]; it may be
/// called from any worker thread.
pub(crate) struct Texts<'e> {
    set: TextSet<'e>,
    notes: Vec<LocalizationNote>,
    languages: [LanguageCount; CATALOG_LANGUAGES.len()],
    placeholders: AtomicUsize,
}

struct LanguageCount {
    language: Language,
    resolved: AtomicUsize,
    unresolved: AtomicUsize,
}

impl LanguageCount {
    const fn new(language: Language) -> Self {
        Self {
            language,
            resolved: AtomicUsize::new(0),
            unresolved: AtomicUsize::new(0),
        }
    }
}

impl<'e> Texts<'e> {
    /// Indexes every language of `sources` in parallel.
    pub(crate) fn parse(sources: &'e TextSources) -> Result<Self, CatalogError> {
        let set = sources.set.index().map_err(|error| match error {
            LocalizationLoadError::Index { path, source } => CatalogError::Texts { path, source },
            other => CatalogError::LoadTexts(other),
        })?;
        let notes = sources.set.notes(&set);
        Ok(Self {
            set,
            notes,
            languages: CATALOG_LANGUAGES.map(LanguageCount::new),
            placeholders: AtomicUsize::new(0),
        })
    }

    /// Returns the text with localisation `key` and its English and German texts, each with `\n`
    /// decoded to a line feed; without a key every part is `None` and nothing is counted, and a
    /// [placeholder key](crate::PLACEHOLDER_KEYS) keeps the key, has no texts and is counted as a
    /// placeholder.
    pub(crate) fn localised<'a>(&self, key: Option<&'a str>) -> LocalisedText<'a>
    where
        'e: 'a,
    {
        let Some(key) = key else {
            return LocalisedText::default();
        };
        if is_placeholder_key(key) {
            self.placeholders.fetch_add(1, Ordering::Relaxed);
            return LocalisedText {
                key: Some(key),
                ..LocalisedText::default()
            };
        }
        LocalisedText {
            key: Some(key),
            en: self.resolve(Language::English, key),
            de: self.resolve(Language::German, key),
        }
    }

    /// Returns a note for every `global.ini` that was read lossily or had lines skipped.
    pub(crate) fn notes(&self) -> &[LocalizationNote] {
        &self.notes
    }

    /// Returns what resolved per language, in the order of [`CATALOG_LANGUAGES`].
    pub(crate) fn summary(&self) -> Vec<TextSummary> {
        let placeholder = self.placeholders.load(Ordering::Relaxed);
        self.languages
            .iter()
            .map(|count| TextSummary {
                language: count.language,
                available: self.set.ini(count.language).is_some(),
                resolved: count.resolved.load(Ordering::Relaxed),
                unresolved: count.unresolved.load(Ordering::Relaxed),
                placeholder,
            })
            .collect()
    }

    fn resolve(&self, language: Language, key: &str) -> Option<Cow<'e, str>> {
        let count = self
            .languages
            .iter()
            .find(|count| count.language == language)?;
        if let Lookup::Found { text, .. } = self.set.lookup(key, language, None) {
            count.resolved.fetch_add(1, Ordering::Relaxed);
            Some(decode_line_breaks(text))
        } else {
            count.unresolved.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}
