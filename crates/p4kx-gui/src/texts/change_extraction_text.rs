/// A text of extracting the changes of one Comparison side, looked up in the Fluent file of the
/// current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeExtractionText {
    /// The action that extracts the changed Entries of the old side; takes `number`.
    ActionOld,
    /// The action that extracts the added and changed Entries of the new side; takes `number`.
    ActionNew,
    /// The scope of the setup for the old side; takes `number`.
    ScopeOld,
    /// The scope of the setup for the new side; takes `number`.
    ScopeNew,
    /// The note that the Filter narrows the scope; takes `filter`.
    ScopeFiltered,
}

impl ChangeExtractionText {
    /// Every text, for checks of the Fluent files.
    pub const ALL: [Self; 5] = [
        Self::ActionOld,
        Self::ActionNew,
        Self::ScopeOld,
        Self::ScopeNew,
        Self::ScopeFiltered,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ActionOld => "change-extraction-action-old",
            Self::ActionNew => "change-extraction-action-new",
            Self::ScopeOld => "change-extraction-scope-old",
            Self::ScopeNew => "change-extraction-scope-new",
            Self::ScopeFiltered => "change-extraction-scope-filtered",
        }
    }
}
