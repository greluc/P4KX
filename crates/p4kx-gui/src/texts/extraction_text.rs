/// A text of the Extraction windows, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtractionText {
    /// The action that extracts the checked Entries.
    ActionSelected,
    /// The action that extracts the shown Entries.
    ActionShown,
    /// The action that extracts every Entry.
    ActionAll,
    /// The title of the setup window.
    SetupTitle,
    /// The heading of the scope choice.
    ScopeHeading,
    /// The checked Entries; takes `number`.
    ScopeSelected,
    /// The shown Entries; takes `number`.
    ScopeShown,
    /// Every Entry; takes `number`.
    ScopeAll,
    /// The heading of the target choice.
    TargetHeading,
    /// A new folder inside the chosen folder.
    TargetNew,
    /// The chosen folder itself.
    TargetExisting,
    /// The name of the new folder; takes `name`.
    TargetNewName,
    /// Shown while no folder is chosen.
    FolderNone,
    /// The action that picks the folder through a folder dialog.
    ChooseFolder,
    /// The heading of the Conflict policy choice.
    PolicyHeading,
    /// Keeps existing files.
    PolicySkip,
    /// Replaces existing files.
    PolicyOverwrite,
    /// Replaces existing files only when they differ.
    PolicyIfChanged,
    /// The switch that converts `CryXml` Entries to XML.
    Conversion,
    /// The action that starts the Extraction.
    Start,
    /// The action that closes a window.
    Close,
    /// The question before a whole-Archive Extraction; takes `number`, `count` and `size`.
    ConfirmAll,
    /// The action that confirms a whole-Archive Extraction.
    ConfirmAllAction,
    /// The action that returns to the choices.
    Back,
    /// The title of the progress window.
    RunningTitle,
    /// The progress; takes `done`, `total` and `percent`.
    Progress,
    /// Shown once the Extraction was asked to stop.
    Cancelling,
    /// The action that stops the Extraction.
    Cancel,
    /// The title of the summary of a complete Extraction.
    SummaryTitle,
    /// The title of the summary of a cancelled Extraction.
    SummaryCancelledTitle,
    /// The title of the summary of an Extraction stopped by a change of the Archive.
    SummaryChangedTitle,
    /// The note that the Archive changed on disk and written files may be stale.
    SummaryChanged,
    /// The counts of the summary; takes `extracted`, `skipped`, `failed` and `cancelled`.
    SummaryCounts,
    /// The counts of the summary with Conversion on; takes `extracted`, `converted`, `skipped`,
    /// `failed` and `cancelled`.
    SummaryCountsConverted,
    /// The target folder; takes `path`.
    SummaryTarget,
    /// The heading of the failed Entries; takes `number`.
    SummaryFailed,
    /// The heading of the skipped Entries; takes `number`.
    SummarySkipped,
    /// The heading of the cancelled Entries; takes `number`.
    SummaryCancelled,
    /// One failed Entry; takes `path` and `reason`.
    SummaryFailure,
    /// The title of the window shown when the Extraction could not run.
    FailedTitle,
    /// The target could not be prepared; takes `path` and `reason`.
    Failed,
    /// The Extraction stopped without a result; takes `path`.
    Stopped,
}

impl ExtractionText {
    /// Every text, for checks of the Fluent files.
    pub const ALL: [Self; 42] = [
        Self::ActionSelected,
        Self::ActionShown,
        Self::ActionAll,
        Self::SetupTitle,
        Self::ScopeHeading,
        Self::ScopeSelected,
        Self::ScopeShown,
        Self::ScopeAll,
        Self::TargetHeading,
        Self::TargetNew,
        Self::TargetExisting,
        Self::TargetNewName,
        Self::FolderNone,
        Self::ChooseFolder,
        Self::PolicyHeading,
        Self::PolicySkip,
        Self::PolicyOverwrite,
        Self::PolicyIfChanged,
        Self::Conversion,
        Self::Start,
        Self::Close,
        Self::ConfirmAll,
        Self::ConfirmAllAction,
        Self::Back,
        Self::RunningTitle,
        Self::Progress,
        Self::Cancelling,
        Self::Cancel,
        Self::SummaryTitle,
        Self::SummaryCancelledTitle,
        Self::SummaryChangedTitle,
        Self::SummaryChanged,
        Self::SummaryCounts,
        Self::SummaryCountsConverted,
        Self::SummaryTarget,
        Self::SummaryFailed,
        Self::SummarySkipped,
        Self::SummaryCancelled,
        Self::SummaryFailure,
        Self::FailedTitle,
        Self::Failed,
        Self::Stopped,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ActionSelected => "extract-selected",
            Self::ActionShown => "extract-shown",
            Self::ActionAll => "extract-all",
            Self::SetupTitle => "extraction-setup-title",
            Self::ScopeHeading => "extraction-scope-heading",
            Self::ScopeSelected => "extraction-scope-selected",
            Self::ScopeShown => "extraction-scope-shown",
            Self::ScopeAll => "extraction-scope-all",
            Self::TargetHeading => "extraction-target-heading",
            Self::TargetNew => "extraction-target-new",
            Self::TargetExisting => "extraction-target-existing",
            Self::TargetNewName => "extraction-target-new-name",
            Self::FolderNone => "extraction-folder-none",
            Self::ChooseFolder => "extraction-choose-folder",
            Self::PolicyHeading => "extraction-policy-heading",
            Self::PolicySkip => "extraction-policy-skip",
            Self::PolicyOverwrite => "extraction-policy-overwrite",
            Self::PolicyIfChanged => "extraction-policy-if-changed",
            Self::Conversion => "extraction-conversion",
            Self::Start => "extraction-start",
            Self::Close => "extraction-close",
            Self::ConfirmAll => "extraction-confirm-all",
            Self::ConfirmAllAction => "extraction-confirm-all-action",
            Self::Back => "extraction-back",
            Self::RunningTitle => "extraction-running-title",
            Self::Progress => "extraction-progress",
            Self::Cancelling => "extraction-cancelling",
            Self::Cancel => "extraction-cancel",
            Self::SummaryTitle => "extraction-summary-title",
            Self::SummaryCancelledTitle => "extraction-summary-cancelled-title",
            Self::SummaryChangedTitle => "extraction-summary-changed-title",
            Self::SummaryChanged => "extraction-summary-changed",
            Self::SummaryCounts => "extraction-summary-counts",
            Self::SummaryCountsConverted => "extraction-summary-counts-converted",
            Self::SummaryTarget => "extraction-summary-target",
            Self::SummaryFailed => "extraction-summary-failed",
            Self::SummarySkipped => "extraction-summary-skipped",
            Self::SummaryCancelled => "extraction-summary-cancelled",
            Self::SummaryFailure => "extraction-summary-failure",
            Self::FailedTitle => "extraction-failed-title",
            Self::Failed => "extraction-failed",
            Self::Stopped => "extraction-stopped",
        }
    }
}
