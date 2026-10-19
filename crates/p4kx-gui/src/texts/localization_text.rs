use p4kx_extraction::Language;

use crate::{TextLanguage, Texts};

/// A text of the localisation tab, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalizationText {
    /// The tab with the table of localisation keys.
    Tab,
    /// The label of the search field.
    Label,
    /// The placeholder of the empty search field.
    Hint,
    /// Shown while the `global.ini` files are read.
    Loading,
    /// Shown when the Archive holds no `global.ini`.
    NoTexts,
    /// Shown when reading failed; takes `reason`.
    Failed,
    /// Shown when reading stopped without a result.
    Stopped,
    /// Shown when the Archive changed on disk.
    ArchiveChanged,
    /// The title before an Archive is open.
    EmptyTitle,
    /// The hint before an Archive is open.
    EmptyHint,
    /// The head of the key column.
    ColumnKey,
    /// The name of a language column, in the interface language.
    LanguageName(Language),
    /// The name of a language this version does not know.
    OtherLanguage,
    /// The mark of a missing translation.
    Missing,
    /// The number of keys; takes `count` and `number`.
    Keys,
    /// The number of keys shown of all keys; takes `count`, `number` and `total`.
    KeysShown,
    /// Shown while the search runs.
    Searching,
    /// Shown when no key or text matches the search.
    NoMatch,
    /// The action that copies the key of the selected row.
    CopyKey,
    /// The action that copies the text of the selected cell.
    CopyText,
    /// The remark above the files that were not read cleanly.
    Notes,
}

impl LocalizationText {
    /// Every text of the localisation tab.
    pub(crate) const ALL: [Self; 31] = [
        Self::Tab,
        Self::Label,
        Self::Hint,
        Self::Loading,
        Self::NoTexts,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::EmptyTitle,
        Self::EmptyHint,
        Self::ColumnKey,
        Self::LanguageName(Language::English),
        Self::LanguageName(Language::German),
        Self::LanguageName(Language::French),
        Self::LanguageName(Language::Italian),
        Self::LanguageName(Language::SpanishSpain),
        Self::LanguageName(Language::SpanishLatinAmerica),
        Self::LanguageName(Language::PortugueseBrazil),
        Self::LanguageName(Language::Japanese),
        Self::LanguageName(Language::Korean),
        Self::LanguageName(Language::ChineseSimplified),
        Self::LanguageName(Language::ChineseTraditional),
        Self::OtherLanguage,
        Self::Missing,
        Self::Keys,
        Self::KeysShown,
        Self::Searching,
        Self::NoMatch,
        Self::CopyKey,
        Self::CopyText,
        Self::Notes,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "localization-tab",
            Self::Label => "localization-label",
            Self::Hint => "localization-hint",
            Self::Loading => "localization-loading",
            Self::NoTexts => "localization-no-texts",
            Self::Failed => "localization-failed",
            Self::Stopped => "localization-stopped",
            Self::ArchiveChanged => "localization-archive-changed",
            Self::EmptyTitle => "localization-empty-title",
            Self::EmptyHint => "localization-empty-hint",
            Self::ColumnKey => "localization-column-key",
            Self::LanguageName(language) => language_id(language),
            Self::OtherLanguage => "localization-language-other",
            Self::Missing => "localization-missing",
            Self::Keys => "localization-keys",
            Self::KeysShown => "localization-keys-shown",
            Self::Searching => "localization-searching",
            Self::NoMatch => "localization-no-match",
            Self::CopyKey => "localization-copy-key",
            Self::CopyText => "localization-copy-text",
            Self::Notes => "localization-notes",
        }
    }
}

const fn language_id(language: Language) -> &'static str {
    match language {
        Language::English => "localization-language-en",
        Language::German => "localization-language-de",
        Language::French => "localization-language-fr",
        Language::Italian => "localization-language-it",
        Language::SpanishSpain => "localization-language-es-ES",
        Language::SpanishLatinAmerica => "localization-language-es-419",
        Language::PortugueseBrazil => "localization-language-pt-BR",
        Language::Japanese => "localization-language-ja",
        Language::Korean => "localization-language-ko",
        Language::ChineseSimplified => "localization-language-zh-Hans",
        Language::ChineseTraditional => "localization-language-zh-Hant",
        _ => "localization-language-other",
    }
}

/// Returns the name of `language` in the interface language; a language this version does not
/// know is named by its code.
pub(crate) fn language_name(texts: &Texts, language: TextLanguage) -> String {
    let known = LocalizationText::LanguageName(language);
    if known.id() == LocalizationText::OtherLanguage.id() {
        return format!(
            "{} ({})",
            texts.get(LocalizationText::OtherLanguage),
            language.code()
        );
    }
    texts.get(known)
}

impl From<LocalizationText> for crate::Text {
    fn from(text: LocalizationText) -> Self {
        Self::Localization(text)
    }
}
