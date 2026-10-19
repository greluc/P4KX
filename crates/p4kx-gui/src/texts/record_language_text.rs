use crate::TextLanguage;

/// A text of the language selection for Record previews, looked up in the Fluent file of the
/// current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordLanguageText {
    /// The menu that chooses the language of the texts next to localisation keys.
    Menu,
    /// The entry that shows the keys alone.
    KeysOnly,
    /// The name of a language of the game's texts, in the language of the interface.
    Name(TextLanguage),
    /// The mark of a key whose language has no text for it.
    Missing,
    /// A localisation key and its text for assistive technology and the tooltip; takes `key` and
    /// `text`.
    KeyRow,
    /// A localisation key without a text for assistive technology and the tooltip; takes `key`.
    KeyRowMissing,
    /// Shown when the texts of a language cannot be read; takes `language` and `reason`.
    LoadFailed,
}

impl RecordLanguageText {
    /// Every text of the language selection.
    pub(crate) const ALL: [Self; 17] = [
        Self::Menu,
        Self::KeysOnly,
        Self::Name(TextLanguage::English),
        Self::Name(TextLanguage::German),
        Self::Name(TextLanguage::French),
        Self::Name(TextLanguage::Italian),
        Self::Name(TextLanguage::SpanishSpain),
        Self::Name(TextLanguage::SpanishLatinAmerica),
        Self::Name(TextLanguage::PortugueseBrazil),
        Self::Name(TextLanguage::Japanese),
        Self::Name(TextLanguage::Korean),
        Self::Name(TextLanguage::ChineseSimplified),
        Self::Name(TextLanguage::ChineseTraditional),
        Self::Missing,
        Self::KeyRow,
        Self::KeyRowMissing,
        Self::LoadFailed,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Menu => "record-language-menu",
            Self::KeysOnly => "record-language-keys-only",
            Self::Name(language) => name_id(language),
            Self::Missing => "record-language-missing",
            Self::KeyRow => "record-language-key-row",
            Self::KeyRowMissing => "record-language-key-row-missing",
            Self::LoadFailed => "record-language-load-failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RecordLanguageText;
    use crate::TextLanguage;

    #[test]
    fn every_language_has_a_name() {
        for language in TextLanguage::ALL {
            assert!(
                RecordLanguageText::ALL.contains(&RecordLanguageText::Name(language)),
                "{language}"
            );
        }
    }

    #[test]
    fn every_name_has_its_own_message() {
        let mut ids: Vec<&str> = RecordLanguageText::ALL.map(RecordLanguageText::id).to_vec();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), RecordLanguageText::ALL.len(), "ids are unique");
    }
}

const fn name_id(language: TextLanguage) -> &'static str {
    match language {
        TextLanguage::German => "record-language-de",
        TextLanguage::French => "record-language-fr",
        TextLanguage::Italian => "record-language-it",
        TextLanguage::SpanishSpain => "record-language-es-es",
        TextLanguage::SpanishLatinAmerica => "record-language-es-419",
        TextLanguage::PortugueseBrazil => "record-language-pt-br",
        TextLanguage::Japanese => "record-language-ja",
        TextLanguage::Korean => "record-language-ko",
        TextLanguage::ChineseSimplified => "record-language-zh-hans",
        TextLanguage::ChineseTraditional => "record-language-zh-hant",
        _ => "record-language-en",
    }
}
