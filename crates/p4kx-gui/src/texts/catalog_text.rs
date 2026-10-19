/// A text of the Basetool catalog entry of the Export menu and its windows, looked up in the
/// Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogText {
    /// The entry of the Export menu that asks for the catalog file.
    Action,
    /// The file type of the save dialog.
    FileFilter,
    /// The title of the progress window.
    RunningTitle,
    /// Shown while DataForge is read.
    Reading,
    /// The progress while Records are checked; takes `done`, `total` and `percent`.
    Progress,
    /// Shown while the file is written; takes `path`.
    Writing,
    /// The title of the summary.
    SummaryTitle,
    /// The heading of the counts per kind.
    KindsHeading,
    /// The counts of one kind; takes `kind`, `written`, `skipped` and `complete` (`yes` or `no`).
    SummaryKind,
    /// The Records left out as development leftovers over every kind; takes `number`.
    SummarySkipped,
    /// The heading of the Records that could not be read; takes `number`.
    SummaryFailed,
    /// One Record that could not be read; takes `path`, `guid` and `reason`.
    SummaryFailure,
    /// The heading of the texts per language.
    TextsHeading,
    /// How the texts resolved in one language; takes `language`, `resolved`, `missing` and
    /// `placeholders`.
    SummaryTexts,
    /// A language whose `global.ini` the Archive lacks; takes `language` and `missing`.
    SummaryTextsUnavailable,
    /// The missing texts over every language; takes `number`.
    SummaryMissing,
    /// The size of the catalog; takes `size` and `limit`.
    SummarySize,
    /// The catalog is close to the upload limit; takes `limit`.
    NearLimit,
    /// The catalog is over the upload limit; takes `limit`.
    OverLimit,
    /// The catalog file; takes `path`.
    SummaryFile,
    /// The title of the window of a cancelled catalog.
    CancelledTitle,
    /// The note that the catalog was cancelled.
    Cancelled,
    /// The title of the window shown when the catalog could not be created.
    FailedTitle,
    /// The catalog could not be created; takes `path` and `reason`.
    Failed,
    /// The catalog stopped without a result; takes `path`.
    Stopped,
    /// The chosen file is the open Archive; takes `path`.
    IsArchive,
}

impl CatalogText {
    /// Every text, for checks of the Fluent files.
    pub const ALL: [Self; 26] = [
        Self::Action,
        Self::FileFilter,
        Self::RunningTitle,
        Self::Reading,
        Self::Progress,
        Self::Writing,
        Self::SummaryTitle,
        Self::KindsHeading,
        Self::SummaryKind,
        Self::SummarySkipped,
        Self::SummaryFailed,
        Self::SummaryFailure,
        Self::TextsHeading,
        Self::SummaryTexts,
        Self::SummaryTextsUnavailable,
        Self::SummaryMissing,
        Self::SummarySize,
        Self::NearLimit,
        Self::OverLimit,
        Self::SummaryFile,
        Self::CancelledTitle,
        Self::Cancelled,
        Self::FailedTitle,
        Self::Failed,
        Self::Stopped,
        Self::IsArchive,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Action => "catalog-action",
            Self::FileFilter => "catalog-file-filter",
            Self::RunningTitle => "catalog-running-title",
            Self::Reading => "catalog-reading",
            Self::Progress => "catalog-progress",
            Self::Writing => "catalog-writing",
            Self::SummaryTitle => "catalog-summary-title",
            Self::KindsHeading => "catalog-kinds-heading",
            Self::SummaryKind => "catalog-summary-kind",
            Self::SummarySkipped => "catalog-summary-skipped",
            Self::SummaryFailed => "catalog-summary-failed",
            Self::SummaryFailure => "catalog-summary-failure",
            Self::TextsHeading => "catalog-texts-heading",
            Self::SummaryTexts => "catalog-summary-texts",
            Self::SummaryTextsUnavailable => "catalog-summary-texts-unavailable",
            Self::SummaryMissing => "catalog-summary-missing",
            Self::SummarySize => "catalog-summary-size",
            Self::NearLimit => "catalog-near-limit",
            Self::OverLimit => "catalog-over-limit",
            Self::SummaryFile => "catalog-summary-file",
            Self::CancelledTitle => "catalog-cancelled-title",
            Self::Cancelled => "catalog-cancelled",
            Self::FailedTitle => "catalog-failed-title",
            Self::Failed => "catalog-failed",
            Self::Stopped => "catalog-stopped",
            Self::IsArchive => "catalog-output-is-archive",
        }
    }
}

impl From<CatalogText> for super::Text {
    fn from(text: CatalogText) -> Self {
        Self::Catalog(text)
    }
}
