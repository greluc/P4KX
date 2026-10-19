/// A text of the language selection for Record previews, looked up in the Fluent file of the
/// current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordLanguageText {
    /// The menu that chooses the language of the texts next to localisation keys.
    Menu,
    /// The entry that shows the keys alone.
    KeysOnly,
    /// Shown in the menu when the Archive holds no texts in the wanted language; takes language.
    Unavailable,
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
    pub(crate) const ALL: [Self; 6] = [
        Self::Menu,
        Self::KeysOnly,
        Self::Unavailable,
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
            Self::Unavailable => "record-language-unavailable",
            Self::KeyRow => "record-language-key-row",
            Self::KeyRowMissing => "record-language-key-row-missing",
            Self::LoadFailed => "record-language-load-failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RecordLanguageText;

    #[test]
    fn every_text_has_its_own_message() {
        let mut ids: Vec<&str> = RecordLanguageText::ALL.map(RecordLanguageText::id).to_vec();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), RecordLanguageText::ALL.len(), "ids are unique");
    }
}
