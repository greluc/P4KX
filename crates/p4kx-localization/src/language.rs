use std::fmt;

/// A language whose texts the Archive holds in a `global.ini`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Language {
    /// English, the language every key is written for first.
    English,
    /// German.
    German,
}

impl Language {
    /// Every language, in the order Exports list them.
    pub const ALL: [Self; 2] = [Self::English, Self::German];

    /// Returns the ISO 639-1 code, e.g. `en`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
        }
    }

    /// Returns the path of the language's `global.ini` Entry in the Archive.
    #[must_use]
    pub const fn global_ini_path(self) -> &'static str {
        match self {
            Self::English => "Data\\Localization\\english\\global.ini",
            Self::German => "Data\\Localization\\german_(germany)\\global.ini",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
