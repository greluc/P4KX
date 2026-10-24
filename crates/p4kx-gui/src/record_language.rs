// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{Language, TextLanguage};

/// What the user chose for the texts Record previews show next to localisation keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordTextChoice {
    /// The keys alone.
    KeysOnly,
    /// The texts of this language's `global.ini`.
    Language(TextLanguage),
}

impl RecordTextChoice {
    const KEYS_ONLY_CODE: &'static str = "none";

    /// Returns the code stored in the settings: `none` or the language code, e.g. `de`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::KeysOnly => Self::KEYS_ONLY_CODE,
            Self::Language(language) => language.code(),
        }
    }

    /// Returns the choice stored as `code`, or `None` for an unknown code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        if code.eq_ignore_ascii_case(Self::KEYS_ONLY_CODE) {
            return Some(Self::KeysOnly);
        }
        TextLanguage::ALL
            .into_iter()
            .find(|language| language.code().eq_ignore_ascii_case(code))
            .map(Self::Language)
    }
}

/// Returns the language whose texts previews would show: the choice, or the language of the
/// interface when nothing was chosen; `None` when the keys alone were chosen.
#[must_use]
pub(crate) const fn wanted(
    choice: Option<RecordTextChoice>,
    interface: Language,
) -> Option<TextLanguage> {
    match choice {
        Some(RecordTextChoice::KeysOnly) => None,
        Some(RecordTextChoice::Language(language)) => Some(language),
        None => Some(match interface {
            Language::German => TextLanguage::German,
            Language::BritishEnglish => TextLanguage::English,
        }),
    }
}

/// Returns the language whose texts previews show: the wanted one, provided the Archive holds
/// it; `None` shows the keys alone.
#[must_use]
pub(crate) fn effective(
    choice: Option<RecordTextChoice>,
    interface: Language,
    present: &[TextLanguage],
) -> Option<TextLanguage> {
    wanted(choice, interface).filter(|language| present.contains(language))
}

#[cfg(test)]
mod tests {
    use super::{RecordTextChoice, effective};
    use crate::{Language, TextLanguage};

    #[test]
    fn every_choice_round_trips_through_its_code() {
        let choices = std::iter::once(RecordTextChoice::KeysOnly)
            .chain(TextLanguage::ALL.map(RecordTextChoice::Language));
        for choice in choices {
            assert_eq!(
                RecordTextChoice::from_code(choice.code()),
                Some(choice),
                "{choice:?}"
            );
        }
        assert_eq!(
            RecordTextChoice::from_code("DE"),
            Some(RecordTextChoice::Language(TextLanguage::German)),
            "case is ignored"
        );
        assert_eq!(RecordTextChoice::from_code("klingon"), None, "unknown code");
    }

    #[test]
    fn an_unchosen_language_follows_the_interface_when_the_archive_holds_it() {
        let present = [TextLanguage::English, TextLanguage::German];

        assert_eq!(
            effective(None, Language::German, &present),
            Some(TextLanguage::German),
            "German interface"
        );
        assert_eq!(
            effective(None, Language::BritishEnglish, &present),
            Some(TextLanguage::English),
            "English interface"
        );
        assert_eq!(
            effective(None, Language::German, &[TextLanguage::English]),
            None,
            "the Archive holds no German"
        );
    }

    #[test]
    fn a_choice_wins_over_the_interface_and_needs_its_file() {
        let present = [TextLanguage::English, TextLanguage::French];
        let french = Some(RecordTextChoice::Language(TextLanguage::French));

        assert_eq!(
            effective(french, Language::German, &present),
            Some(TextLanguage::French),
            "chosen"
        );
        assert_eq!(
            effective(french, Language::German, &[TextLanguage::English]),
            None,
            "missing file"
        );
        assert_eq!(
            effective(Some(RecordTextChoice::KeysOnly), Language::German, &present),
            None,
            "keys only"
        );
    }
}
