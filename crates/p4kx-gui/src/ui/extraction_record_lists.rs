// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::Ui;

use super::extraction_outcome::{failure_lines, list};
use crate::{ExtractionSummary, ExtractionText as T, Texts};

/// Shows the lists of the Records of the DataForge Entry: the ones not written and the ones
/// written under another file name.
pub(super) fn show(ui: &mut Ui, texts: &Texts, summary: &ExtractionSummary) {
    let heading = T::SummaryRecordsFailed;
    let lines = failure_lines(
        ui,
        texts,
        (summary, heading),
        summary.record_failures(),
        |failure| {
            let args = [
                ("path", failure.record_path.clone().into()),
                ("guid", failure.guid.clone().into()),
                ("reason", failure.reason.clone().into()),
            ];
            (
                texts.format(T::SummaryRecordFailure, &args),
                failure.reason.len(),
            )
        },
    );
    list(
        ui,
        texts,
        (heading, &lines.lines, Some(&lines.reasons)),
        true,
    );
    let heading = T::SummaryRecordsRenamed;
    let renamed = failure_lines(
        ui,
        texts,
        (summary, heading),
        summary.records_renamed(),
        |note| {
            let args = [
                ("path", note.record_path.clone().into()),
                ("guid", note.guid.clone().into()),
                ("reason", note.reason.clone().into()),
            ];
            (texts.format(T::SummaryRecordRenamed, &args), 0)
        },
    );
    list(ui, texts, (heading, &renamed.lines, None), true);
}
