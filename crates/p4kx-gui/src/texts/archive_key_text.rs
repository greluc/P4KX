/// A text of the Archive key dialog and the entry that opens it, looked up in the Fluent file of
/// the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveKeyText {
    /// The entry of the Archive menu that opens the dialog.
    Menu,
    /// The title of the dialog.
    Title,
    /// What the dialog asks for when an encrypted Entry was needed.
    Needed,
    /// What the dialog says when the key in use does not fit the Archive.
    Rejected,
    /// What the dialog explains when it was opened from the menu.
    Intro,
    /// The label of the key field.
    Label,
    /// The hint inside the empty key field.
    Hint,
    /// The accessible name of the switch that shows or hides the key typed.
    Show,
    /// A key is stored in the profile; takes `masked`.
    Stored,
    /// No key is stored in the profile.
    NotStored,
    /// The action that stores the key.
    Save,
    /// The action that closes the dialog without a change.
    Cancel,
    /// The action that removes the stored key.
    Remove,
    /// The key typed is too short or too long; takes `digits`.
    ErrorLength,
    /// The key typed has a character that is no hexadecimal digit; takes `position`.
    ErrorCharacter,
    /// The key typed does not decrypt the Archive.
    ErrorWrong,
    /// The key could not be checked against the Archive; takes `reason`.
    ErrorCheck,
    /// The key could not be stored in the profile; takes `reason`.
    ErrorStore,
    /// The stored key could not be removed; takes `reason`.
    ErrorRemove,
}

impl ArchiveKeyText {
    /// Every text, for checks of the Fluent files.
    pub const ALL: [Self; 19] = [
        Self::Menu,
        Self::Title,
        Self::Needed,
        Self::Rejected,
        Self::Intro,
        Self::Label,
        Self::Hint,
        Self::Show,
        Self::Stored,
        Self::NotStored,
        Self::Save,
        Self::Cancel,
        Self::Remove,
        Self::ErrorLength,
        Self::ErrorCharacter,
        Self::ErrorWrong,
        Self::ErrorCheck,
        Self::ErrorStore,
        Self::ErrorRemove,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Menu => "key-menu",
            Self::Title => "key-title",
            Self::Needed => "key-needed",
            Self::Rejected => "key-rejected",
            Self::Intro => "key-intro",
            Self::Label => "key-label",
            Self::Hint => "key-hint",
            Self::Show => "key-show",
            Self::Stored => "key-stored",
            Self::NotStored => "key-not-stored",
            Self::Save => "key-save",
            Self::Cancel => "key-cancel",
            Self::Remove => "key-remove",
            Self::ErrorLength => "key-error-length",
            Self::ErrorCharacter => "key-error-character",
            Self::ErrorWrong => "key-error-wrong",
            Self::ErrorCheck => "key-error-check",
            Self::ErrorStore => "key-error-store",
            Self::ErrorRemove => "key-error-remove",
        }
    }
}

impl From<ArchiveKeyText> for super::Text {
    fn from(text: ArchiveKeyText) -> Self {
        Self::Key(text)
    }
}
