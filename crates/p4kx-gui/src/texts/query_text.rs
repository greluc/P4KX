/// A text of the Query tab, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryText {
    /// The tab that runs Queries.
    Tab,
    /// The label of the Query editor.
    Label,
    /// The placeholder of the empty Query editor.
    Hint,
    /// The action that runs the Query.
    Run,
    /// The action that stops the running Query.
    Cancel,
    /// Shown while DataForge is read.
    Loading,
    /// Shown while the Records are selected.
    Selecting,
    /// The progress; takes `done`, `total` and `percent`.
    Progress,
    /// Shown once the Query was cancelled.
    Cancelled,
    /// Shown when the Query failed; takes `reason`.
    Failed,
    /// Shown when the Query stopped without a result.
    Stopped,
    /// Shown when the Archive changed on disk.
    ArchiveChanged,
    /// The error of a Query that cannot be run; takes `line`, `column` and `reason`.
    Invalid,
    /// The title before an Archive is open.
    EmptyTitle,
    /// The hint before an Archive is open.
    EmptyHint,
    /// Shown in place of the result before a Query ran.
    Prompt,
    /// Shown when no Record matches.
    NoRows,
    /// The number of rows; takes `count` and `number`.
    Rows,
    /// The number of rows and failed Records; takes `count`, `number` and `failed`.
    RowsFailed,
    /// The unresolved References of the rows, after their number; takes `count` and `number`.
    Unresolved,
    /// The action that exports the result as CSV.
    ExportCsv,
    /// The action that exports the result as JSON.
    ExportJson,
    /// The file type filter of the CSV export dialog.
    FilterCsv,
    /// The file type filter of the JSON export dialog.
    FilterJson,
    /// Shown while the result is exported; takes `path`.
    Exporting,
    /// Shown once the result was exported; takes `count`, `number` and `path`.
    Exported,
    /// Shown when the export failed; takes `path` and `reason`.
    ExportFailed,
    /// The action that stops the running export.
    ExportCancel,
    /// Shown once the export was cancelled; takes `path`.
    ExportCancelled,
    /// The page shown; takes `page` and `pages`.
    Page,
    /// The button that shows the first page.
    PageFirst,
    /// The button that shows the previous page.
    PagePrevious,
    /// The button that shows the next page.
    PageNext,
    /// The button that shows the last page.
    PageLast,
}

impl QueryText {
    /// Every Query text.
    pub(crate) const ALL: [Self; 34] = [
        Self::Tab,
        Self::Label,
        Self::Hint,
        Self::Run,
        Self::Cancel,
        Self::Loading,
        Self::Selecting,
        Self::Progress,
        Self::Cancelled,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::Invalid,
        Self::EmptyTitle,
        Self::EmptyHint,
        Self::Prompt,
        Self::NoRows,
        Self::Rows,
        Self::RowsFailed,
        Self::Unresolved,
        Self::ExportCsv,
        Self::ExportJson,
        Self::FilterCsv,
        Self::FilterJson,
        Self::Exporting,
        Self::Exported,
        Self::ExportFailed,
        Self::ExportCancel,
        Self::ExportCancelled,
        Self::Page,
        Self::PageFirst,
        Self::PagePrevious,
        Self::PageNext,
        Self::PageLast,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "query-tab",
            Self::Label => "query-label",
            Self::Hint => "query-hint",
            Self::Run => "query-run",
            Self::Cancel => "query-cancel",
            Self::Loading => "query-loading",
            Self::Selecting => "query-selecting",
            Self::Progress => "query-progress",
            Self::Cancelled => "query-cancelled",
            Self::Failed => "query-failed",
            Self::Stopped => "query-stopped",
            Self::ArchiveChanged => "query-archive-changed",
            Self::Invalid => "query-invalid",
            Self::EmptyTitle => "query-empty-title",
            Self::EmptyHint => "query-empty-hint",
            Self::Prompt => "query-prompt",
            Self::NoRows => "query-no-rows",
            Self::Rows => "query-rows",
            Self::RowsFailed => "query-rows-failed",
            Self::Unresolved => "query-unresolved",
            Self::ExportCsv => "query-export-csv",
            Self::ExportJson => "query-export-json",
            Self::FilterCsv => "query-filter-csv",
            Self::FilterJson => "query-filter-json",
            Self::Exporting => "query-exporting",
            Self::Exported => "query-exported",
            Self::ExportFailed => "query-export-failed",
            Self::ExportCancel => "query-export-cancel",
            Self::ExportCancelled => "query-export-cancelled",
            Self::Page => "query-page",
            Self::PageFirst => "query-page-first",
            Self::PagePrevious => "query-page-previous",
            Self::PageNext => "query-page-next",
            Self::PageLast => "query-page-last",
        }
    }
}

impl From<QueryText> for crate::Text {
    fn from(text: QueryText) -> Self {
        Self::Query(text)
    }
}
