// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the Convert folder windows, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FolderConversionText {
    /// The action in the Archive menu.
    Action,
    /// The title of the progress window.
    RunningTitle,
    /// The progress; takes `done`, `total` and `percent`.
    Progress,
    /// Shown while the folder is listed.
    Scanning,
    /// The title of the summary of a finished conversion.
    SummaryTitle,
    /// The title of the summary of a cancelled conversion.
    SummaryCancelledTitle,
    /// The counts of a conversion; takes `extracted`, `converted`, `skipped`, `failed`,
    /// `cancelled` and `left`.
    SummaryCounts,
    /// The input folder; takes `path`.
    SummaryInput,
    /// The title of the window of a conversion that failed.
    FailedTitle,
    /// The conversion of a folder failed; takes `path` and `reason`.
    Failed,
    /// The conversion stopped without a result; takes `path`.
    Stopped,
}

impl FolderConversionText {
    /// Every text, for checks of the Fluent files.
    pub(crate) const ALL: [Self; 11] = [
        Self::Action,
        Self::RunningTitle,
        Self::Progress,
        Self::Scanning,
        Self::SummaryTitle,
        Self::SummaryCancelledTitle,
        Self::SummaryCounts,
        Self::SummaryInput,
        Self::FailedTitle,
        Self::Failed,
        Self::Stopped,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Action => "folder-conversion-action",
            Self::RunningTitle => "folder-conversion-running-title",
            Self::Progress => "folder-conversion-progress",
            Self::Scanning => "folder-conversion-scanning",
            Self::SummaryTitle => "folder-conversion-summary-title",
            Self::SummaryCancelledTitle => "folder-conversion-summary-cancelled-title",
            Self::SummaryCounts => "folder-conversion-summary-counts",
            Self::SummaryInput => "folder-conversion-summary-input",
            Self::FailedTitle => "folder-conversion-failed-title",
            Self::Failed => "folder-conversion-failed",
            Self::Stopped => "folder-conversion-stopped",
        }
    }
}

impl From<FolderConversionText> for super::Text {
    fn from(text: FolderConversionText) -> Self {
        Self::FolderConversion(text)
    }
}
