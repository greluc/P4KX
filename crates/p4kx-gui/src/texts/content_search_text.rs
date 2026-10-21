/// A text of the Search tab, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentSearchText {
    /// The tab that searches the contents of the Archive.
    Tab,
    /// The label of the search field.
    Label,
    /// The placeholder of the empty search field.
    Hint,
    /// The action that starts the search.
    Run,
    /// The action that stops the running search.
    Cancel,
    /// The label of the sources to search.
    SourcesLabel,
    /// The source: the values of the Records.
    SourceRecords,
    /// The source: the localisation texts.
    SourceLocalization,
    /// The source: the lines of text and CryXml Entries.
    SourceEntries,
    /// The title before an Archive is open.
    EmptyTitle,
    /// The hint before an Archive is open.
    EmptyHint,
    /// Shown in place of the hits before a search ran.
    Prompt,
    /// Shown while the search runs.
    Searching,
    /// Shown once the search was cancelled.
    Cancelled,
    /// Shown when the search failed; takes `reason`.
    Failed,
    /// Shown when the search stopped without a result.
    Stopped,
    /// Shown when the Archive changed on disk.
    ArchiveChanged,
    /// Shown when nothing matches; takes `text`.
    NoMatch,
    /// Shown when the listed source has no hit although another has.
    NoneHere,
    /// The number of hits of a source, shown as the count of its segment; takes `number`.
    Segment,
    /// The number of hits of a source with more hits than were kept; takes `number`.
    SegmentMore,
    /// The number of hits and the time; takes `count`, `number` and `seconds`.
    Hits,
    /// The note that more matches exist than were kept; takes `number`.
    More,
    /// The note that Entries were not searched; takes `count`, `number`, `binary`, `large` and
    /// `unreadable`.
    Skipped,
    /// The note that encrypted Entries were not searched; takes `count` and `number`.
    KeyRequired,
    /// The action that opens the Archive key dialog from that note.
    KeyButton,
    /// The note that Records were not searched because the Archive holds no DataForge.
    NoDataForge,
    /// The head of the column of the record path.
    ColumnRecord,
    /// The head of the column of the field path.
    ColumnField,
    /// The head of the column of the excerpt.
    ColumnText,
    /// The head of the column of the language.
    ColumnLanguage,
    /// The head of the column of the localisation key.
    ColumnKey,
    /// The head of the column of the Entry path.
    ColumnEntry,
    /// The head of the column of the line number.
    ColumnLine,
    /// The accessible name of a hit; takes `place` and `text`.
    Row,
}

impl ContentSearchText {
    /// Every text of the Search tab.
    pub(crate) const ALL: [Self; 35] = [
        Self::Tab,
        Self::Label,
        Self::Hint,
        Self::Run,
        Self::Cancel,
        Self::SourcesLabel,
        Self::SourceRecords,
        Self::SourceLocalization,
        Self::SourceEntries,
        Self::EmptyTitle,
        Self::EmptyHint,
        Self::Prompt,
        Self::Searching,
        Self::Cancelled,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::NoMatch,
        Self::NoneHere,
        Self::Segment,
        Self::SegmentMore,
        Self::Hits,
        Self::More,
        Self::Skipped,
        Self::KeyRequired,
        Self::KeyButton,
        Self::NoDataForge,
        Self::ColumnRecord,
        Self::ColumnField,
        Self::ColumnText,
        Self::ColumnLanguage,
        Self::ColumnKey,
        Self::ColumnEntry,
        Self::ColumnLine,
        Self::Row,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "content-search-tab",
            Self::Label => "content-search-label",
            Self::Hint => "content-search-hint",
            Self::Run => "content-search-run",
            Self::Cancel => "content-search-cancel",
            Self::SourcesLabel => "content-search-sources",
            Self::SourceRecords => "content-search-source-records",
            Self::SourceLocalization => "content-search-source-localization",
            Self::SourceEntries => "content-search-source-entries",
            Self::EmptyTitle => "content-search-empty-title",
            Self::EmptyHint => "content-search-empty-hint",
            Self::Prompt => "content-search-prompt",
            Self::Searching => "content-search-searching",
            Self::Cancelled => "content-search-cancelled",
            Self::Failed => "content-search-failed",
            Self::Stopped => "content-search-stopped",
            Self::ArchiveChanged => "content-search-archive-changed",
            Self::NoMatch => "content-search-no-match",
            Self::NoneHere => "content-search-none-here",
            Self::Segment => "content-search-segment",
            Self::SegmentMore => "content-search-segment-more",
            Self::Hits => "content-search-hits",
            Self::More => "content-search-more",
            Self::Skipped => "content-search-skipped",
            Self::KeyRequired => "content-search-key-required",
            Self::KeyButton => "content-search-key-button",
            Self::NoDataForge => "content-search-no-dataforge",
            Self::ColumnRecord => "content-search-column-record",
            Self::ColumnField => "content-search-column-field",
            Self::ColumnText => "content-search-column-text",
            Self::ColumnLanguage => "content-search-column-language",
            Self::ColumnKey => "content-search-column-key",
            Self::ColumnEntry => "content-search-column-entry",
            Self::ColumnLine => "content-search-column-line",
            Self::Row => "content-search-row",
        }
    }
}
