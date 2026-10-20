/// A text of the "Referenced by" tab of the Record preview, looked up in the Fluent file of the
/// current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncomingText {
    /// The preview tab that lists the Records referring to the selected Record.
    Tab,
    /// The column of the field that holds the Reference.
    HeadField,
    /// The column of the Record that holds the Reference.
    HeadRecord,
    /// Shown while the Records are found; takes `path`.
    Loading,
    /// Shown when the Records cannot be found; takes `path` and `reason`.
    Failed,
    /// Shown when finding them stopped without a result; takes `path`.
    Stopped,
    /// Shown instead of the list once the Archive changed on disk.
    ArchiveChanged,
    /// Shown instead of the list when no Record refers to the Record.
    None,
    /// The count line; takes `references` and `records`, the texts of the two counts.
    Summary,
    /// The number of References; takes `count` and the formatted `number`.
    ReferenceCount,
    /// The number of Records; takes `count` and the formatted `number`.
    RecordCount,
    /// The note above a list cut short; takes `shown` and `total`.
    Truncated,
    /// The note that Records could not be indexed or the index is full, so the list may miss
    /// References.
    Incomplete,
}

impl IncomingText {
    /// Every text of the "Referenced by" tab.
    pub(crate) const ALL: [Self; 13] = [
        Self::Tab,
        Self::HeadField,
        Self::HeadRecord,
        Self::Loading,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::None,
        Self::Summary,
        Self::ReferenceCount,
        Self::RecordCount,
        Self::Truncated,
        Self::Incomplete,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "preview-tab-incoming",
            Self::HeadField => "incoming-head-field",
            Self::HeadRecord => "incoming-head-record",
            Self::Loading => "incoming-loading",
            Self::Failed => "incoming-failed",
            Self::Stopped => "incoming-stopped",
            Self::ArchiveChanged => "incoming-archive-changed",
            Self::None => "incoming-none",
            Self::Summary => "incoming-summary",
            Self::ReferenceCount => "incoming-reference-count",
            Self::RecordCount => "incoming-record-count",
            Self::Truncated => "incoming-truncated",
            Self::Incomplete => "incoming-incomplete",
        }
    }
}
