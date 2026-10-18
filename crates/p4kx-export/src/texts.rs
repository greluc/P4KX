use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_extraction::{Archive, GlobalIni, Language, LocalizationEntry, LocalizationLoadError};

use crate::contract::LocalisedText;
use crate::{CatalogError, TextSummary};

/// The `global.ini` Entries of English and German, `None` for a language the Archive lacks.
pub(crate) struct TextSources {
    english: Option<LocalizationEntry>,
    german: Option<LocalizationEntry>,
}

impl TextSources {
    /// Reads the `global.ini` of English and German from `archive`.
    pub(crate) fn read(archive: &Archive) -> Result<Self, CatalogError> {
        Ok(Self {
            english: read(archive, Language::English)?,
            german: read(archive, Language::German)?,
        })
    }
}

fn read(archive: &Archive, language: Language) -> Result<Option<LocalizationEntry>, CatalogError> {
    match LocalizationEntry::read(archive, language) {
        Ok(entry) => Ok(Some(entry)),
        Err(LocalizationLoadError::Missing { .. }) => Ok(None),
        Err(error) => Err(CatalogError::LoadTexts(error)),
    }
}

/// Resolves localisation keys to English and German texts and counts what resolves.
///
/// Every kind turns its keys into [`LocalisedText`]s through [`Texts::localised`]; it may be
/// called from any worker thread.
pub(crate) struct Texts<'e> {
    english: LanguageTexts<'e>,
    german: LanguageTexts<'e>,
}

impl<'e> Texts<'e> {
    /// Indexes both languages of `sources` in parallel.
    pub(crate) fn parse(sources: &'e TextSources) -> Result<Self, CatalogError> {
        let (english, german) = rayon::join(
            || LanguageTexts::parse(Language::English, sources.english.as_ref()),
            || LanguageTexts::parse(Language::German, sources.german.as_ref()),
        );
        Ok(Self {
            english: english?,
            german: german?,
        })
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
            en: self.english.resolve(key),
            de: self.german.resolve(key),
        }
    }

    /// Returns what resolved per language, English first.
    pub(crate) fn summary(&self) -> Vec<TextSummary> {
        vec![self.english.summary(), self.german.summary()]
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
