// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Label, ScrollArea, Ui};
use fluent_bundle::FluentValue;

use super::hex_view;
use crate::theme::desktop::HEX_ROW_HEIGHT;
use crate::theme::palette;
use crate::widgets::{frames, scroll_region};
use crate::{
    PREVIEW_SIZE_LIMIT, Preview, PreviewContents, PreviewStatus, PreviewTab, ScrollText, Text,
    Texts, format,
};

/// Shows what the Text and Hex tabs show of the Entry read: the data, or why there are none.
pub(super) fn status_body(
    ui: &mut Ui,
    texts: &Texts,
    preview: &Preview,
    (path, size): (&str, u64),
    tab: PreviewTab,
) {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    frames::preview_body().show(ui, |ui| match preview.status() {
        PreviewStatus::Loading => {
            ui.horizontal(|ui| {
                crate::widgets::indicator::spinner(ui, crate::theme::desktop::SPINNER_INLINE);
                ui.label(texts.format(Text::PreviewLoading, &[path_arg()]));
            });
        }
        PreviewStatus::TooLarge => {
            ui.label(texts.format(
                Text::PreviewTooLarge,
                &[
                    path_arg(),
                    ("size", format::size(size, language).into()),
                    ("limit", format::size(PREVIEW_SIZE_LIMIT, language).into()),
                ],
            ));
        }
        PreviewStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.format(Text::PreviewStopped, &[path_arg()]),
                |reason| {
                    texts.format(
                        Text::PreviewFailed,
                        &[path_arg(), ("reason", reason.to_owned().into())],
                    )
                },
            );
            ui.colored_label(palette::DANGER_TEXT, text);
        }
        status @ (PreviewStatus::ArchiveChanged
        | PreviewStatus::KeyRequired
        | PreviewStatus::KeyRejected) => note(ui, texts, status, path),
        PreviewStatus::Ready(contents) => ready(ui, texts, contents, tab),
    });
}

/// Shows the note that stands for the preview of an Entry that cannot be read until the Archive is
/// reloaded or has the right key.
fn note(ui: &mut Ui, texts: &Texts, status: PreviewStatus<'_>, path: &str) {
    let path = [("path", FluentValue::from(path.to_owned()))];
    let (colour, text) = match status {
        PreviewStatus::KeyRequired => (
            palette::WARNING,
            texts.format(Text::PreviewKeyRequired, &path),
        ),
        PreviewStatus::KeyRejected => (
            palette::DANGER_TEXT,
            texts.format(Text::PreviewKeyRejected, &path),
        ),
        _ => (palette::WARNING, texts.get(Text::PreviewArchiveChanged)),
    };
    ui.colored_label(colour, text);
}

fn ready(ui: &mut Ui, texts: &Texts, contents: &PreviewContents, tab: PreviewTab) {
    if contents.is_truncated() {
        let language = texts.language();
        let shown = u64::try_from(contents.bytes().len()).unwrap_or(u64::MAX);
        let note = texts.format(
            Text::PreviewTruncated,
            &[
                ("shown", format::number(shown, language).into()),
                ("total", format::number(contents.size(), language).into()),
            ],
        );
        ui.colored_label(palette::WARNING, note);
    }
    match tab {
        PreviewTab::Hex => hex_view::hex_rows(ui, texts, contents),
        _ => text_rows(ui, texts, contents),
    }
}

fn text_rows(ui: &mut Ui, texts: &Texts, contents: &PreviewContents) {
    let area = ScrollArea::both()
        .id_salt("preview-text")
        .auto_shrink(false);
    let id = ui.scope_id().with("preview-text");
    scroll_region::show(
        ui,
        (id, || texts.get(ScrollText::Text)),
        HEX_ROW_HEIGHT + ui.spacing().item_spacing.y,
        area,
        |area, ui| {
            area.show_rows(ui, HEX_ROW_HEIGHT, contents.line_count(), |ui, rows| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for index in rows {
                    ui.add(Label::new(contents.line(index)).extend());
                }
            })
        },
    );
}
