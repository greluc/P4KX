use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_extraction::{Archive, GlobalIni, Language, LocalizationEntry, LocalizationLoadError};
use rayon::prelude::*;

use crate::contract::LocalisedText;
use crate::{CatalogError, TextSummary};

/// The `global.ini` Entry of every language in [`Language::ALL`], `None` for a language the
/// Archive lacks.
pub(crate) struct TextSources {
    entries: Vec<(Language, Option<LocalizationEntry>)>,
}

impl TextSources {
    /// Reads the `global.ini` of every language from `archive`.
    pub(crate) fn read(archive: &Archive) -> Result<Self, CatalogError> {
        let entries = Language::ALL
            .into_iter()
            .map(|language| Ok((language, read_if_present(archive, language)?)))
            .collect::<Result<_, CatalogError>>()?;
        Ok(Self { entries })
    }
}

fn read_if_present(
    archive: &Archive,
    language: Language,
) -> Result<Option<LocalizationEntry>, CatalogError> {
    match LocalizationEntry::read(archive, language) {
        Ok(entry) => Ok(Some(entry)),
        Err(LocalizationLoadError::Missing { .. }) => Ok(None),
        Err(error) => Err(CatalogError::LoadTexts(error)),
    }
}

/// Resolves localisation keys in every language and counts what resolves.
///
/// Every kind turns its keys into [`LocalisedText`]s through [`Texts::localised`]; it may be
/// called from any worker thread.
pub(crate) struct Texts<'e> {
    languages: Vec<LanguageTexts<'e>>,
}

impl<'e> Texts<'e> {
    /// Indexes every language of `sources` in parallel.
    pub(crate) fn parse(sources: &'e TextSources) -> Result<Self, CatalogError> {
        let languages = sources
            .entries
            .par_iter()
            .map(|(language, entry)| LanguageTexts::parse(*language, entry.as_ref()))
            .collect::<Result<_, _>>()?;
        Ok(Self { languages })
    }

    /// Returns the text with localisation `key` and its English and German texts; without a key
    /// every part is `None` and nothing is counted.
    pub(crate) fn localised<'a>(&self, key: Option<&'a str>) -> LocalisedText<'a>
    where
        'e: 'a,
    {
        let Some(key) = key else {
            return LocalisedText::default();
        };
        LocalisedText {
            key: Some(key),
            en: self.resolve(Language::English, key),
            de: self.resolve(Language::German, key),
        }
    }

    /// Returns what resolved per language, in the order of [`Language::ALL`].
    pub(crate) fn summary(&self) -> Vec<TextSummary> {
        self.languages.iter().map(LanguageTexts::summary).collect()
    }

    fn resolve(&self, language: Language, key: &str) -> Option<&'e str> {
        self.languages
            .iter()
            .find(|texts| texts.language == language)
            .and_then(|texts| texts.resolve(key))
    }
}

struct LanguageTexts<'e> {
    language: Language,
    ini: Option<GlobalIni<'e>>,
    resolved: AtomicUsize,
    unresolved: AtomicUsize,
}

impl<'e> LanguageTexts<'e> {
    fn parse(
        language: Language,
        source: Option<&'e LocalizationEntry>,
    ) -> Result<Self, CatalogError> {
        let ini = source
            .map(|entry| {
                entry.parse().map_err(|source| CatalogError::Texts {
                    path: entry.path().to_owned(),
                    source,
                })
            })
            .transpose()?;
        Ok(Self {
            language,
            ini,
            resolved: AtomicUsize::new(0),
            unresolved: AtomicUsize::new(0),
        })
    }

    fn resolve(&self, key: &str) -> Option<&'e str> {
        let text = self.ini.as_ref().and_then(|ini| ini.resolve(key));
        let counter = if text.is_some() {
            &self.resolved
        } else {
            &self.unresolved
        };
        counter.fetch_add(1, Ordering::Relaxed);
        text
    }

    fn summary(&self) -> TextSummary {
        TextSummary {
            language: self.language,
            available: self.ini.is_some(),
            resolved: self.resolved.load(Ordering::Relaxed),
            unresolved: self.unresolved.load(Ordering::Relaxed),
        }
    }
}
