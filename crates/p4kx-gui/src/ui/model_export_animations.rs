// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::model_export_notes::number;
use crate::{AnimationNoteKind, ModelExportSummary, ModelExportText as T, Texts};

/// Returns the line with the animations and channels the file holds, or `None` when the export
/// was not asked to write animations.
pub(super) fn written_line(texts: &Texts, summary: &ModelExportSummary) -> Option<String> {
    if !summary.animations_asked() {
        return None;
    }
    let channels = summary
        .written()
        .animations
        .iter()
        .map(|animation| animation.channels)
        .sum();
    Some(texts.format(
        T::SummaryAnimations,
        &[
            (
                "written",
                number(texts, summary.animations_written()).into(),
            ),
            ("channels", number(texts, channels).into()),
        ],
    ))
}

/// Returns the heading and the lines of the animations that were not written.
pub(super) fn unread_lines(texts: &Texts, summary: &ModelExportSummary) -> (String, Vec<String>) {
    let heading = texts.format(
        T::SummaryAnimationsUnread,
        &[(
            "number",
            number(texts, summary.unread_animations_total()).into(),
        )],
    );
    let lines = summary
        .unread_animations()
        .iter()
        .map(|note| {
            let source = ("source", note.source.clone().into());
            let name = note.name.clone().unwrap_or_default();
            match (&note.kind, &note.name) {
                (AnimationNoteKind::NoBoneMatched { skipped_tracks }, _) => texts.format(
                    T::SummaryAnimationUnmatched,
                    &[
                        ("name", name.into()),
                        source,
                        (
                            "tracks",
                            u64::try_from(*skipped_tracks).unwrap_or(u64::MAX).into(),
                        ),
                        ("number", number(texts, *skipped_tracks).into()),
                    ],
                ),
                (AnimationNoteKind::Unread { reason }, None) => texts.format(
                    T::SummaryAnimationFileUnread,
                    &[source, ("reason", reason.clone().into())],
                ),
                (AnimationNoteKind::Unread { reason }, Some(_)) => texts.format(
                    T::SummaryAnimationUnread,
                    &[
                        ("name", name.into()),
                        source,
                        ("reason", reason.clone().into()),
                    ],
                ),
            }
        })
        .collect();
    (heading, lines)
}
