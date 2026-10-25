// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// The accessible name of a view the keyboard scrolls, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollText {
    /// The lines of the Text preview.
    Text,
    /// The rows of the Hex preview.
    Hex,
    /// The picture of the Image preview.
    Image,
    /// The side-by-side rows of the line diff.
    Diff,
    /// The field rows of a Record change.
    Fields,
    /// The field lines of the Record preview.
    Lines,
    /// The rows of the Localisation table.
    Localization,
    /// The rows of the saved Queries.
    SavedQueries,
    /// The Records referring to the selected Record.
    Incoming,
    /// The embedded WEMs of a soundbank.
    Soundbank,
    /// The hits of the Search tab.
    Hits,
    /// The summary of the DataForge tab.
    Summary,
}

impl ScrollText {
    /// Every accessible name of a scrollable view.
    pub(crate) const ALL: [Self; 12] = [
        Self::Text,
        Self::Hex,
        Self::Image,
        Self::Diff,
        Self::Fields,
        Self::Lines,
        Self::Localization,
        Self::SavedQueries,
        Self::Incoming,
        Self::Soundbank,
        Self::Hits,
        Self::Summary,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Text => "scroll-text",
            Self::Hex => "scroll-hex",
            Self::Image => "scroll-image",
            Self::Diff => "scroll-diff",
            Self::Fields => "scroll-fields",
            Self::Lines => "scroll-lines",
            Self::Localization => "scroll-localization",
            Self::SavedQueries => "scroll-saved-queries",
            Self::Incoming => "scroll-incoming",
            Self::Soundbank => "scroll-soundbank",
            Self::Hits => "scroll-hits",
            Self::Summary => "scroll-summary",
        }
    }
}
