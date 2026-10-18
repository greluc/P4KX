/// A text of the Record search, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordSearchText {
    /// The label of the Record search field.
    Label,
    /// The placeholder of the Record search field.
    Hint,
    /// Shown in place of matches while the search field is empty.
    Prompt,
    /// Shown when no Record matches.
    NoMatches,
    /// The number of matching Records; takes `count` and `number`.
    Matches,
}

impl RecordSearchText {
    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Label => "record-search-label",
            Self::Hint => "record-search-hint",
            Self::Prompt => "record-search-prompt",
            Self::NoMatches => "record-search-no-matches",
            Self::Matches => "record-search-matches",
        }
    }
}

impl From<RecordSearchText> for crate::Text {
    fn from(text: RecordSearchText) -> Self {
        Self::DataForge(super::DataForgeText::Search(text))
    }
}
