use egui::Ui;
use fluent_bundle::FluentValue;

use super::preview::path_line;
use super::record_lines::{self, List};
use super::record_preview::{RecordStep, heading};
use crate::theme::desktop::SPINNER_INLINE;
use crate::theme::palette;
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::{
    Browser, IncomingPreview, IncomingStatus, IncomingText, IncomingView, ScrollText, Text, Texts,
    format,
};

const SALT: &str = "incoming-lines";
const HEADS: [Text; 2] = [
    Text::Incoming(IncomingText::HeadField),
    Text::Incoming(IncomingText::HeadRecord),
];

/// Shows the "Referenced by" tab for the selected Record: its record path, the count line and
/// one line per Reference with a link to the Record that holds it; returns the step a link asks
/// for.
pub(super) fn show(ui: &mut Ui, texts: &Texts, browser: &Browser) -> Option<RecordStep> {
    let (path, name) = heading(browser);
    path_line(ui, path, name);
    let preview = browser.incoming_preview();
    let status = preview.map_or(IncomingStatus::Loading, IncomingPreview::status);
    let generation = preview.map_or(0, IncomingPreview::generation);
    if let IncomingStatus::Ready(view) = status {
        return ready(ui, texts, view, generation);
    }
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    frames::preview_body().show(ui, |ui| match status {
        IncomingStatus::Loading => {
            ui.horizontal(|ui| {
                spinner(ui, SPINNER_INLINE);
                ui.label(texts.format(IncomingText::Loading, &[path_arg()]));
            });
        }
        IncomingStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.format(IncomingText::Stopped, &[path_arg()]),
                |reason| {
                    let reason = ("reason", FluentValue::from(reason.to_owned()));
                    texts.format(IncomingText::Failed, &[path_arg(), reason])
                },
            );
            ui.colored_label(palette::DANGER_TEXT, text);
        }
        IncomingStatus::ArchiveChanged => {
            ui.colored_label(palette::WARNING, texts.get(IncomingText::ArchiveChanged));
        }
        IncomingStatus::Ready(_) => {}
    });
    None
}

fn ready(ui: &mut Ui, texts: &Texts, view: &IncomingView, generation: u64) -> Option<RecordStep> {
    if view.lines().is_empty() {
        frames::preview_body().show(ui, |ui| {
            ui.colored_label(palette::GRAY_2_TEXT, texts.get(IncomingText::None));
        });
        return None;
    }
    frames::preview_body().show(ui, |ui| {
        ui.colored_label(palette::GRAY_1, summary(texts, view));
        if view.is_truncated() {
            let shown = ("shown", count(texts, view.lines().len()).into());
            let total = ("total", count(texts, view.references()).into());
            ui.colored_label(
                palette::GRAY_2_TEXT,
                texts.format(IncomingText::Truncated, &[shown, total]),
            );
        }
    });
    if view.is_incomplete() {
        frames::warning_alert(ui, |ui| {
            ui.colored_label(palette::GRAY_1, texts.get(IncomingText::Incomplete));
        });
    }
    let list = List {
        lines: view.lines(),
        heads: HEADS,
        scroll: ScrollText::Incoming,
        salt: SALT,
    };
    record_lines::show_list(ui, texts, &list, generation)
}

fn summary(texts: &Texts, view: &IncomingView) -> String {
    let counted = |text: IncomingText, count: usize| {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        let args = [
            ("count", FluentValue::from(count)),
            ("number", format::number(count, texts.language()).into()),
        ];
        texts.format(text, &args)
    };
    let references = counted(IncomingText::ReferenceCount, view.references());
    let records = counted(IncomingText::RecordCount, view.records());
    texts.format(
        IncomingText::Summary,
        &[
            ("references", references.into()),
            ("records", records.into()),
        ],
    )
}

fn count(texts: &Texts, count: usize) -> String {
    format::number(u64::try_from(count).unwrap_or(u64::MAX), texts.language())
}
