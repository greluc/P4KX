/// A language of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// German, `de-DE`.
    German,
    /// British English, `en-GB`.
    BritishEnglish,
}

impl Language {
    /// Every interface language.
    pub const ALL: [Self; 2] = [Self::German, Self::BritishEnglish];

    /// Returns the BCP 47 tag, as stored in the settings file.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::German => "de-DE",
            Self::BritishEnglish => "en-GB",
        }
    }

    /// Returns the language with the tag `tag`, ignoring case, or `None` for another tag.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.tag().eq_ignore_ascii_case(tag))
    }

    /// Returns German for a German system locale such as `de-AT` and British English otherwise.
    #[must_use]
    pub fn from_locale(locale: &str) -> Self {
        let primary = locale.split(['-', '_']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("de") {
            Self::German
        } else {
            Self::BritishEnglish
        }
    }

    /// Returns the language of the stored tag, or the one of the system locale when the tag is
    /// missing or unknown.
    #[must_use]
    pub fn resolve(stored: Option<&str>) -> Self {
        stored.and_then(Self::from_tag).unwrap_or_else(|| {
            sys_locale::get_locale()
                .map_or(Self::BritishEnglish, |locale| Self::from_locale(&locale))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn tags_round_trip_ignoring_case() {
        for language in Language::ALL {
            assert_eq!(
                Language::from_tag(language.tag()),
                Some(language),
                "own tag"
            );
        }
        assert_eq!(Language::from_tag("DE-de"), Some(Language::German), "case");
        assert_eq!(Language::from_tag("fr-FR"), None, "unknown tag");
    }

    #[test]
    fn german_locales_choose_german_and_all_others_british_english() {
        for locale in ["de", "de-DE", "de-AT", "de_CH"] {
            assert_eq!(Language::from_locale(locale), Language::German, "{locale}");
        }
        for locale in ["en-US", "en-GB", "fr-FR", "", "dk"] {
            assert_eq!(
                Language::from_locale(locale),
                Language::BritishEnglish,
                "{locale}"
            );
        }
    }

    #[test]
    fn a_stored_tag_wins_over_the_system() {
        assert_eq!(Language::resolve(Some("de-DE")), Language::German, "German");
        assert_eq!(
            Language::resolve(Some("en-GB")),
            Language::BritishEnglish,
            "English"
        );
    }
}
