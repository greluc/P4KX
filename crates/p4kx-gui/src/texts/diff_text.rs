/// A text of the line diff of the Comparison tab, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffText {
    /// The action that moves to the previous hunk.
    Previous,
    /// The action that moves to the next hunk.
    Next,
    /// The hunk navigated to; takes `current` and `total`.
    Position,
    /// The head of the old side.
    HeadOld,
    /// The head of the new side.
    HeadNew,
    /// The lines a hunk covers on one side; takes `first` and `last`.
    Hunk,
    /// A hunk that covers no line on one side.
    HunkEmpty,
    /// Shown while the line diff is made; takes `path`.
    Loading,
    /// Shown instead of the diff of a binary Entry; takes `path`.
    Binary,
    /// Shown instead of the diff of an Entry too large to diff; takes `path`, `size` and `lines`.
    TooLarge,
    /// Shown when the line diff failed; takes `path` and `reason`.
    Failed,
    /// Shown when the line diff stopped without a result; takes `path`.
    Stopped,
    /// Shown when the lines are equal although the bytes differ.
    Equal,
    /// The accessible name of an unchanged line; takes `line` and `text`.
    LineContext,
    /// The accessible name of a removed line; takes `line` and `text`.
    LineRemoved,
    /// The accessible name of an added line; takes `line` and `text`.
    LineAdded,
    /// The marker of a removed line.
    MarkerRemoved,
    /// The marker of an added line.
    MarkerAdded,
}

impl DiffText {
    /// Every text of the line diff.
    pub const ALL: [Self; 18] = [
        Self::Previous,
        Self::Next,
        Self::Position,
        Self::HeadOld,
        Self::HeadNew,
        Self::Hunk,
        Self::HunkEmpty,
        Self::Loading,
        Self::Binary,
        Self::TooLarge,
        Self::Failed,
        Self::Stopped,
        Self::Equal,
        Self::LineContext,
        Self::LineRemoved,
        Self::LineAdded,
        Self::MarkerRemoved,
        Self::MarkerAdded,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Previous => "diff-previous",
            Self::Next => "diff-next",
            Self::Position => "diff-position",
            Self::HeadOld => "diff-head-old",
            Self::HeadNew => "diff-head-new",
            Self::Hunk => "diff-hunk",
            Self::HunkEmpty => "diff-hunk-empty",
            Self::Loading => "diff-loading",
            Self::Binary => "diff-binary",
            Self::TooLarge => "diff-too-large",
            Self::Failed => "diff-failed",
            Self::Stopped => "diff-stopped",
            Self::Equal => "diff-equal",
            Self::LineContext => "diff-line-context",
            Self::LineRemoved => "diff-line-removed",
            Self::LineAdded => "diff-line-added",
            Self::MarkerRemoved => "diff-marker-removed",
            Self::MarkerAdded => "diff-marker-added",
        }
    }
}
