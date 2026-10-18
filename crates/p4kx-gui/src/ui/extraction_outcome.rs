use std::time::Duration;

use egui::{CollapsingHeader, Id, ProgressBar, ScrollArea, Ui, Window};
use fluent_bundle::FluentValue;
use p4kx_extraction::Progress;

use super::extraction::number;
use super::row_height;
use crate::{ExtractionStatus, ExtractionSummary, ExtractionText as T, FailedEntry, Model, Texts};

const REFRESH: Duration = Duration::from_millis(100);
const LIST_HEIGHT: f32 = 180.0;
const WINDOW: &str = "extraction-run";

pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(extraction) = model.extraction() else {
        return;
    };
    let texts = model.texts();
    let path = extraction.folder().display().to_string();
    let mut cancel = false;
    let mut open = true;
    match extraction.status() {
        ExtractionStatus::Running(progress) => {
            let cancelling = extraction.is_cancelled();
            Window::new(texts.get(T::RunningTitle))
                .id(Id::unique(WINDOW))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    cancel = running(ui, texts, progress, cancelling);
                });
            ui.ctx().request_repaint_after(REFRESH);
        }
        ExtractionStatus::Finished(summary) => {
            let title = if summary.archive_changed() {
                T::SummaryChangedTitle
            } else if summary.cancelled().is_empty() {
                T::SummaryTitle
            } else {
                T::SummaryCancelledTitle
            };
            outcome_window(ui, texts, title, &mut open, |ui| {
                finished(ui, texts, summary);
            });
        }
        ExtractionStatus::Failed(reason) => {
            let text = match reason {
                Some(reason) => texts.format(
                    T::Failed,
                    &[("path", path.into()), ("reason", reason.to_owned().into())],
                ),
                None => texts.format(T::Stopped, &[("path", path.into())]),
            };
            outcome_window(ui, texts, T::FailedTitle, &mut open, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, text);
            });
        }
    }
    if cancel {
        model.cancel_extraction();
    }
    if !open {
        model.close_extraction();
    }
}

fn outcome_window(ui: &Ui, texts: &Texts, title: T, open: &mut bool, body: impl FnOnce(&mut Ui)) {
    let mut close = false;
    Window::new(texts.get(title))
        .id(Id::unique(WINDOW))
        .collapsible(false)
        .open(open)
        .default_width(560.0)
        .show(ui.ctx(), |ui| {
            body(ui);
            ui.separator();
            close = ui.button(texts.get(T::Close)).clicked();
        });
    if close {
        *open = false;
    }
}

fn running(ui: &mut Ui, texts: &Texts, progress: Progress, cancelling: bool) -> bool {
    let fraction = if progress.bytes_total == 0 {
        0.0
    } else {
        ratio(progress.bytes_done, progress.bytes_total)
    };
    let percent = (fraction * 100.0).floor();
    ui.label(texts.format(
        T::Progress,
        &[
            ("done", number(texts, progress.entries_done)),
            ("total", number(texts, progress.entries_total)),
            ("percent", FluentValue::from(f64::from(percent))),
        ],
    ));
    ui.add(ProgressBar::new(fraction).desired_width(360.0));
    if cancelling {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(texts.get(T::Cancelling));
        });
        false
    } else {
        ui.button(texts.get(T::Cancel)).clicked()
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a progress bar needs no more than f32 precision"
)]
fn ratio(done: u64, total: u64) -> f32 {
    (done as f32 / total as f32).clamp(0.0, 1.0)
}

fn finished(ui: &mut Ui, texts: &Texts, summary: &ExtractionSummary) {
    if summary.archive_changed() {
        ui.colored_label(ui.visuals().error_fg_color, texts.get(T::SummaryChanged));
    }
    let mut counts = vec![
        ("extracted", number(texts, summary.extracted())),
        ("skipped", number(texts, summary.skipped().len())),
        ("failed", number(texts, summary.failures().len())),
        ("cancelled", number(texts, summary.cancelled().len())),
    ];
    let text = summary.converted().map_or(T::SummaryCounts, |converted| {
        counts.push(("converted", number(texts, converted)));
        T::SummaryCountsConverted
    });
    ui.label(texts.format(text, &counts));
    let target = summary.target().display().to_string();
    ui.label(texts.format(T::SummaryTarget, &[("path", target.into())]));
    let failures = failure_lines(texts, summary.failures());
    list(ui, texts, T::SummaryFailed, &failures, true);
    let unconverted = failure_lines(texts, summary.unconverted());
    list(ui, texts, T::SummaryUnconverted, &unconverted, true);
    list(ui, texts, T::SummarySkipped, summary.skipped(), false);
    list(ui, texts, T::SummaryCancelled, summary.cancelled(), false);
}

fn failure_lines(texts: &Texts, failures: &[FailedEntry]) -> Vec<String> {
    failures
        .iter()
        .map(|failure| {
            texts.format(
                T::SummaryFailure,
                &[
                    ("path", failure.path.clone().into()),
                    ("reason", failure.reason.clone().into()),
                ],
            )
        })
        .collect()
}

fn list(ui: &mut Ui, texts: &Texts, heading: T, lines: &[String], open: bool) {
    if lines.is_empty() {
        return;
    }
    let title = texts.format(heading, &[("number", number(texts, lines.len()))]);
    CollapsingHeader::new(title)
        .id_salt(heading.id())
        .default_open(open)
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt(heading.id())
                .max_height(LIST_HEIGHT)
                .show_rows(ui, row_height(ui), lines.len(), |ui, rows| {
                    for line in lines.get(rows).unwrap_or_default() {
                        ui.label(line);
                    }
                });
        });
}
