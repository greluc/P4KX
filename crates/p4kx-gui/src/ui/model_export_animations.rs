// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{ModelExportSummary, ModelExportText as T, Texts, format};

fn number(texts: &Texts, value: usize) -> String {
    format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language())
}

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
            let reason = ("reason", note.reason.clone().into());
            note.name.as_ref().map_or_else(
                || {
                    texts.format(
                        T::SummaryAnimationFileUnread,
                        &[source.clone(), reason.clone()],
                    )
                },
                |name| {
                    texts.format(
                        T::SummaryAnimationUnread,
                        &[
                            ("name", name.clone().into()),
                            source.clone(),
                            reason.clone(),
                        ],
                    )
                },
            )
        })
        .collect();
    (heading, lines)
}
