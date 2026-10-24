// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the About window, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AboutText {
    /// The action and window title.
    Title,
    /// The version of the program; takes `version`.
    Version,
    /// The copyright line, identical in every language.
    Copyright,
    /// The licence of the program.
    Licence,
    /// The statement that this is an unofficial fan project.
    FanProject,
    /// The statement that no game data ships and only the user's installation is read.
    NoGameData,
    /// The statement that the user must comply with the Star Citizen EULA and Terms of Service.
    Responsibility,
    /// The statement about the ownership of content that is not the authors'.
    Owners,
    /// The trademark notice of Cloud Imperium, identical in every language.
    Trademark,
}

impl AboutText {
    /// Every text of the About window.
    pub(crate) const ALL: [Self; 9] = [
        Self::Title,
        Self::Version,
        Self::Copyright,
        Self::Licence,
        Self::FanProject,
        Self::NoGameData,
        Self::Responsibility,
        Self::Owners,
        Self::Trademark,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Title => "about-title",
            Self::Version => "about-version",
            Self::Copyright => "about-copyright",
            Self::Licence => "about-licence",
            Self::FanProject => "about-fan-project",
            Self::NoGameData => "about-no-game-data",
            Self::Responsibility => "about-responsibility",
            Self::Owners => "about-owners",
            Self::Trademark => "about-trademark",
        }
    }
}

impl From<AboutText> for super::Text {
    fn from(text: AboutText) -> Self {
        Self::About(text)
    }
}
