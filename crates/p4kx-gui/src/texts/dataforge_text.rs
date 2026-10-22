mod search_text;

pub use search_text::RecordSearchText;

/// A text of the DataForge node and the DataForge tab, looked up in the Fluent file of the
/// current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataForgeText {
    /// The name of the DataForge node in the path tree.
    Node,
    /// The short note on the node while DataForge loads.
    NodeLoading,
    /// The preview tab that shows the DataForge summary.
    Tab,
    /// Shown while DataForge loads; takes `path`.
    Loading,
    /// DataForge has a version this P4KX does not read; takes `version`.
    Unsupported,
    /// DataForge cannot be read; takes `path` and `reason`.
    Failed,
    /// Loading stopped without a result; takes `path`.
    Stopped,
    /// Shown instead of DataForge once the Archive changed on disk.
    ArchiveChanged,
    /// The key of the DataForge version.
    Version,
    /// The key of the number of Records.
    Records,
    /// The key of the number of structs.
    Structs,
    /// The key of the number of enums.
    Enums,
    /// The heading above the first Records.
    FirstRecords,
    /// The type column.
    ColumnType,
    /// The record path column.
    ColumnRecordPath,
    /// A text of the Record search.
    Search(RecordSearchText),
}

impl DataForgeText {
    /// Every DataForge text.
    pub(crate) const ALL: [Self; 20] = [
        Self::Node,
        Self::NodeLoading,
        Self::Tab,
        Self::Loading,
        Self::Unsupported,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::Version,
        Self::Records,
        Self::Structs,
        Self::Enums,
        Self::FirstRecords,
        Self::ColumnType,
        Self::ColumnRecordPath,
        Self::Search(RecordSearchText::Label),
        Self::Search(RecordSearchText::Hint),
        Self::Search(RecordSearchText::Prompt),
        Self::Search(RecordSearchText::NoMatches),
        Self::Search(RecordSearchText::Matches),
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Node => "dataforge-node",
            Self::NodeLoading => "dataforge-node-loading",
            Self::Tab => "preview-tab-dataforge",
            Self::Loading => "dataforge-loading",
            Self::Unsupported => "dataforge-unsupported",
            Self::Failed => "dataforge-failed",
            Self::Stopped => "dataforge-stopped",
            Self::ArchiveChanged => "dataforge-archive-changed",
            Self::Version => "dataforge-version",
            Self::Records => "dataforge-records",
            Self::Structs => "dataforge-structs",
            Self::Enums => "dataforge-enums",
            Self::FirstRecords => "dataforge-first-records",
            Self::ColumnType => "column-type",
            Self::ColumnRecordPath => "column-record-path",
            Self::Search(text) => text.id(),
        }
    }
}
