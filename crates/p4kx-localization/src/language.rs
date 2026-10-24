// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// A language whose texts the Archive holds in a `global.ini`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Language {
    /// English, the language every key is written for first.
    English,
    /// German.
    German,
    /// French (France).
    French,
    /// Italian (Italy).
    Italian,
    /// Spanish (Spain).
    SpanishSpain,
    /// Spanish (Latin America).
    SpanishLatinAmerica,
    /// Portuguese (Brazil).
    PortugueseBrazil,
    /// Japanese (Japan).
    Japanese,
    /// Korean (South Korea).
    Korean,
    /// Chinese, simplified.
    ChineseSimplified,
    /// Chinese, traditional.
    ChineseTraditional,
}

impl Language {
    /// Every language, in the order Exports list them.
    pub const ALL: [Self; 11] = [
        Self::English,
        Self::German,
        Self::French,
        Self::Italian,
        Self::SpanishSpain,
        Self::SpanishLatinAmerica,
        Self::PortugueseBrazil,
        Self::Japanese,
        Self::Korean,
        Self::ChineseSimplified,
        Self::ChineseTraditional,
    ];

    /// Returns the language code: the ISO 639-1 code, with the region or script where the game
    /// has several variants, e.g. `de`, `es-ES`, `es-419` or `zh-Hans`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
            Self::French => "fr",
            Self::Italian => "it",
            Self::SpanishSpain => "es-ES",
            Self::SpanishLatinAmerica => "es-419",
            Self::PortugueseBrazil => "pt-BR",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::ChineseSimplified => "zh-Hans",
            Self::ChineseTraditional => "zh-Hant",
        }
    }

    /// Returns the name of the folder below `Data\Localization` that holds the language's
    /// `global.ini`, e.g. `german_(germany)`.
    #[must_use]
    pub const fn folder(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::German => "german_(germany)",
            Self::French => "french_(france)",
            Self::Italian => "italian_(italy)",
            Self::SpanishSpain => "spanish_(spain)",
            Self::SpanishLatinAmerica => "spanish_(latin_america)",
            Self::PortugueseBrazil => "portuguese_(brazil)",
            Self::Japanese => "japanese_(japan)",
            Self::Korean => "korean_(south_korea)",
            Self::ChineseSimplified => "chinese_(simplified)",
            Self::ChineseTraditional => "chinese_(traditional)",
        }
    }

    /// Returns the path of the language's `global.ini` Entry in the Archive.
    #[must_use]
    pub const fn global_ini_path(self) -> &'static str {
        match self {
            Self::English => "Data\\Localization\\english\\global.ini",
            Self::German => "Data\\Localization\\german_(germany)\\global.ini",
            Self::French => "Data\\Localization\\french_(france)\\global.ini",
            Self::Italian => "Data\\Localization\\italian_(italy)\\global.ini",
            Self::SpanishSpain => "Data\\Localization\\spanish_(spain)\\global.ini",
            Self::SpanishLatinAmerica => "Data\\Localization\\spanish_(latin_america)\\global.ini",
            Self::PortugueseBrazil => "Data\\Localization\\portuguese_(brazil)\\global.ini",
            Self::Japanese => "Data\\Localization\\japanese_(japan)\\global.ini",
            Self::Korean => "Data\\Localization\\korean_(south_korea)\\global.ini",
            Self::ChineseSimplified => "Data\\Localization\\chinese_(simplified)\\global.ini",
            Self::ChineseTraditional => "Data\\Localization\\chinese_(traditional)\\global.ini",
        }
    }

    /// Returns the language whose `global.ini` Entry has `path`, ignoring ASCII case and the
    /// separator kind; `None` for any other path.
    #[must_use]
    pub fn from_global_ini_path(path: &str) -> Option<Self> {
        let mut parts = path.split(['\\', '/']);
        let (Some(data), Some(localization), Some(folder), Some(file), None) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            return None;
        };
        if !data.eq_ignore_ascii_case("Data")
            || !localization.eq_ignore_ascii_case("Localization")
            || !file.eq_ignore_ascii_case("global.ini")
        {
            return None;
        }
        Self::ALL
            .into_iter()
            .find(|language| language.folder().eq_ignore_ascii_case(folder))
    }

    fn name(self) -> &'static str {
        let folder = self.folder();
        folder.split_once("_(").map_or(folder, |(name, _)| name)
    }

    fn primary_subtag(self) -> &'static str {
        let code = self.code();
        code.split_once('-').map_or(code, |(primary, _)| primary)
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// A text names no language.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown language {input:?}; use one of: {}", known_codes())]
pub struct ParseLanguageError {
    input: String,
}

fn known_codes() -> String {
    let codes: Vec<&str> = Language::ALL.into_iter().map(Language::code).collect();
    codes.join(", ")
}

impl FromStr for Language {
    type Err = ParseLanguageError;

    /// Parses a [language code](Language::code) or a [folder name](Language::folder), ignoring
    /// ASCII case and treating `_` like `-`; the language name without region (`german`) and the
    /// ISO 639-1 code (`pt`) also work when exactly one language has them.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let wanted = input.trim().replace('_', "-");
        let exact = |language: &Self| {
            language.code().eq_ignore_ascii_case(&wanted)
                || language
                    .folder()
                    .replace('_', "-")
                    .eq_ignore_ascii_case(&wanted)
        };
        if let Some(language) = Self::ALL.into_iter().find(exact) {
            return Ok(language);
        }
        let only = |matches: &dyn Fn(&Self) -> bool| {
            let mut found = Self::ALL.into_iter().filter(|language| matches(language));
            match (found.next(), found.next()) {
                (Some(language), None) => Some(language),
                _ => None,
            }
        };
        only(&|language| language.name().eq_ignore_ascii_case(&wanted))
            .or_else(|| only(&|language| language.primary_subtag().eq_ignore_ascii_case(&wanted)))
            .ok_or_else(|| ParseLanguageError {
                input: input.to_owned(),
            })
    }
}
