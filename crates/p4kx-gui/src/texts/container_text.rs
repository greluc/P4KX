use super::Text;

/// A text of the containers in the path tree, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContainerText {
    /// The note of a container while it opens.
    Opening,
    /// The note of a container that cannot be opened.
    Unreadable,
    /// The full message of a container that cannot be opened; takes `name` and `reason`.
    UnreadableDetail,
    /// The note of a container not opened because the open containers hold too much memory.
    OverBudget,
    /// The full message of that; takes `name` and `limit`.
    OverBudgetDetail,
    /// The note of a container whose opening stopped without a result.
    Stopped,
    /// The full message of that; takes `name`.
    StoppedDetail,
    /// The note of a container once the Archive changed on disk.
    ArchiveChanged,
    /// The full message of that; takes `name`.
    ArchiveChangedDetail,
    /// The status bar's number of Entries of the shown container; takes `count` and `number`.
    StatusEntries,
}

impl ContainerText {
    /// Every text of the containers.
    pub(crate) const ALL: [Self; 10] = [
        Self::Opening,
        Self::Unreadable,
        Self::UnreadableDetail,
        Self::OverBudget,
        Self::OverBudgetDetail,
        Self::Stopped,
        Self::StoppedDetail,
        Self::ArchiveChanged,
        Self::ArchiveChangedDetail,
        Self::StatusEntries,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Opening => "container-opening",
            Self::Unreadable => "container-unreadable",
            Self::UnreadableDetail => "container-unreadable-detail",
            Self::OverBudget => "container-over-budget",
            Self::OverBudgetDetail => "container-over-budget-detail",
            Self::Stopped => "container-stopped",
            Self::StoppedDetail => "container-stopped-detail",
            Self::ArchiveChanged => "container-archive-changed",
            Self::ArchiveChangedDetail => "container-archive-changed-detail",
            Self::StatusEntries => "container-status-entries",
        }
    }
}

impl From<ContainerText> for Text {
    fn from(text: ContainerText) -> Self {
        Self::Container(text)
    }
}
