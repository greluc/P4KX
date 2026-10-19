use p4kx_dataforge::{Resolution, TextResolver};
use p4kx_localization::{Language, Lookup, TextSet, decode_line_breaks};

/// The texts of one language of a [`TextSet`], to write into Record XML and Record JSON.
///
/// A key without a text in the language is missing; a placeholder key stays alone. The `\n`
/// escapes of the `global.ini` become line feeds.
#[derive(Debug, Clone, Copy)]
pub struct LanguageTexts<'t, 'a> {
    texts: &'t TextSet<'a>,
    language: Language,
}

impl<'t, 'a> LanguageTexts<'t, 'a> {
    /// Resolves in `language` with the files of `texts`.
    #[must_use]
    pub const fn new(texts: &'t TextSet<'a>, language: Language) -> Self {
        Self { texts, language }
    }

    /// Returns the language keys are resolved in.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }
}

impl TextResolver for LanguageTexts<'_, '_> {
    fn resolve<'r>(&'r self, key: &str) -> Resolution<'r> {
        match self.texts.lookup(key, self.language, None) {
            Lookup::Found { text, .. } => Resolution::Text(decode_line_breaks(text)),
            Lookup::Placeholder => Resolution::Keep,
            _ => Resolution::Missing,
        }
    }
}
