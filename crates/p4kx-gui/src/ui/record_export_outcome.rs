use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use egui::{Id, ScrollArea, Ui};
use fluent_bundle::FluentValue;

use super::extraction::number;
use super::extraction_outcome::{decorate, list_row, ratio};
use crate::theme::desktop::{
    MODAL_WIDE_WIDTH, MODAL_WIDTH, ROW_HEIGHT, SECTION_LIST_HEIGHT, SPINNER_INLINE,
};
use crate::theme::palette;
use crate::theme::tokens::{SPACE_2, SPACE_3};
use crate::widgets::button::Kind;
use crate::widgets::frames::{self, Tone};
use crate::widgets::indicator;
use crate::widgets::rows::section_header;
use crate::{
    ExtractionText, Language, Model, RecordExportProgress, RecordExportStage, RecordExportStatus,
    RecordExportSummary, RecordExportText as T, Text, Texts,
};

const REFRESH: Duration = Duration::from_millis(100);
const WINDOW: &str = "record-export-run";

/// Shows the progress of a running Record export, or the summary once it ended.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(run) = model.record_export() else {
        return;
    };
    let texts = model.texts();
    let mut cancel = false;
    let mut close = false;
    match run.status() {
        RecordExportStatus::Running(progress) => {
            let cancelling = run.is_cancelled();
            let title = texts.get(T::RunningTitle);
            let shown = frames::window(&title, Tone::Normal)
                .id(Id::unique(WINDOW))
                .collapsible(false)
                .resizable(false)
                .default_width(MODAL_WIDTH)
                .show(ui.ctx(), |ui| {
                    frames::head(ui, (&title, Tone::Normal), false, None);
                    cancel = running(ui, texts, progress, cancelling);
                });
            decorate(ui, shown, Tone::Normal);
            ui.ctx().request_repaint_after(REFRESH);
        }
        RecordExportStatus::Finished(summary) => {
            let laid = Laid::get(ui, texts, summary);
            let title = if summary.was_cancelled() {
                T::SummaryCancelledTitle
            } else {
                T::SummaryTitle
            };
            close = outcome_window(ui, texts, (WINDOW, title.into(), Tone::Normal), |ui| {
                finished(ui, &laid);
            });
        }
        RecordExportStatus::Failed(reason) => {
            let path = ("path", run.folder().display().to_string().into());
            let text = match reason {
                Some(reason) => {
                    texts.format(T::Failed, &[path, ("reason", reason.to_owned().into())])
                }
                None => texts.format(T::Stopped, &[path]),
            };
            close = outcome_window(
                ui,
                texts,
                (WINDOW, T::FailedTitle.into(), Tone::Danger),
                |ui| {
                    ui.colored_label(palette::DANGER_TEXT, text);
                },
            );
        }
    }
    if cancel {
        model.cancel_record_export();
    }
    if close {
        model.close_record_export();
    }
}

/// Shows the non-modal outcome window `id` titled `title` with `body` and a Close CTA; returns
/// whether it was closed.
pub(super) fn outcome_window(
    ui: &Ui,
    texts: &Texts,
    (id, title, tone): (&'static str, Text, Tone),
    body: impl FnOnce(&mut Ui),
) -> bool {
    let mut close = false;
    let title = texts.get(title);
    let close_dialog = texts.get(Text::CloseDialog);
    let shown = frames::window(&title, tone)
        .id(Id::unique(id))
        .collapsible(false)
        .default_width(MODAL_WIDE_WIDTH)
        .show(ui.ctx(), |ui| {
            let closed = frames::head(ui, (&title, tone), false, Some(&close_dialog));
            body(ui);
            frames::foot_rule(ui);
            let pressed = frames::foot_action(ui, &texts.get(ExtractionText::Close), Kind::Cta);
            close = pressed || closed;
        });
    decorate(ui, shown, tone);
    close
}

fn running(ui: &mut Ui, texts: &Texts, progress: RecordExportProgress, cancelling: bool) -> bool {
    let fraction = match progress.stage {
        RecordExportStage::Writing if progress.total > 0 => {
            ratio(progress.done as u64, progress.total as u64)
        }
        _ => 0.0,
    };
    match progress.stage {
        RecordExportStage::Reading | RecordExportStage::Selecting => {
            let text = if progress.stage == RecordExportStage::Reading {
                T::Reading
            } else {
                T::Selecting
            };
            ui.horizontal(|ui| {
                indicator::spinner(ui, SPINNER_INLINE);
                ui.label(texts.get(text));
            });
        }
        RecordExportStage::Writing => {
            let percent = (fraction * 100.0).floor();
            ui.label(texts.format(
                T::Progress,
                &[
                    ("done", number(texts, progress.done.min(progress.total))),
                    ("total", number(texts, progress.total)),
                    ("percent", FluentValue::from(f64::from(percent))),
                ],
            ));
        }
    }
    ui.add_space(SPACE_3);
    let width = ui.available_width().max(frames::MODAL_BODY_WIDTH);
    indicator::progress(ui, fraction, cancelling, width);
    frames::foot_rule(ui);
    if cancelling {
        ui.horizontal(|ui| {
            indicator::spinner(ui, SPINNER_INLINE);
            ui.label(texts.get(ExtractionText::Cancelling));
        });
        false
    } else {
        frames::foot_action(ui, &texts.get(ExtractionText::Cancel), Kind::Ghost)
    }
}

/// The texts of one summary, formatted once per summary and language.
#[derive(Clone)]
struct Laid {
    key: (usize, usize, usize, usize, Language),
    notes: Arc<[(String, egui::Color32)]>,
    failures: Arc<[String]>,
    reasons: Arc<[usize]>,
    skipped: Arc<[String]>,
    failed_heading: String,
    skipped_heading: String,
}

impl Laid {
    fn get(ui: &Ui, texts: &Texts, summary: &RecordExportSummary) -> Self {
        let key = (
            std::ptr::from_ref(summary).addr(),
            summary.written(),
            summary.failures().len(),
            summary.skipped().len(),
            texts.language(),
        );
        let id = Id::unique("p4kx-record-export-summary");
        if let Some(cached) = ui.data(|data| data.get_temp::<Self>(id))
            && cached.key == key
        {
            return cached;
        }
        let laid = Self::new(key, texts, summary);
        ui.data_mut(|data| data.insert_temp(id, laid.clone()));
        laid
    }

    fn new(
        key: (usize, usize, usize, usize, Language),
        texts: &Texts,
        summary: &RecordExportSummary,
    ) -> Self {
        let failures: Arc<[String]> = summary
            .failures()
            .iter()
            .map(|failure| {
                texts.format(
                    T::SummaryFailure,
                    &[
                        ("path", failure.record_path.clone().into()),
                        ("guid", failure.guid.clone().into()),
                        ("reason", failure.reason.clone().into()),
                    ],
                )
            })
            .collect();
        let reasons = failures
            .iter()
            .zip(summary.failures())
            .map(|(line, failure)| line.len().saturating_sub(failure.reason.len()))
            .collect();
        Self {
            key,
            notes: notes(texts, summary).into(),
            failures,
            reasons,
            skipped: summary.skipped().into(),
            failed_heading: texts.format(
                T::SummaryFailed,
                &[("number", number(texts, summary.failures().len()))],
            ),
            skipped_heading: texts.format(
                T::SummarySkipped,
                &[("number", number(texts, summary.skipped().len()))],
            ),
        }
    }
}

/// Returns the lines above the lists of a summary, each with its colour.
fn notes(texts: &Texts, summary: &RecordExportSummary) -> Vec<(String, egui::Color32)> {
    let mut notes = Vec::new();
    if summary.was_cancelled() {
        notes.push((texts.get(T::SummaryCancelled), palette::WARNING));
    } else if summary.selected() == 0 {
        notes.push((texts.get(T::SummaryNoMatch), palette::GRAY_2_TEXT));
    }
    let counts = [
        ("written", number(texts, summary.written())),
        ("skipped", number(texts, summary.skipped().len())),
        ("failed", number(texts, summary.failures().len())),
        ("cancelled", number(texts, summary.cancelled())),
    ];
    notes.push((texts.format(T::SummaryCounts, &counts), palette::GRAY_1));
    let errors = [
        ("erroneous", number(texts, summary.records_with_errors())),
        ("errors", number(texts, summary.error_values())),
    ];
    notes.push((texts.format(T::SummaryErrors, &errors), palette::GRAY_1));
    let Some(target) = summary
        .file()
        .or_else(|| summary.target().map(Path::to_path_buf))
        .map(|target| target.display().to_string())
    else {
        return notes;
    };
    if let Some(reason) = summary.error() {
        let args = [
            ("path", target.clone().into()),
            ("reason", reason.to_owned().into()),
        ];
        notes.push((
            texts.format(T::SummaryFileFailed, &args),
            palette::DANGER_TEXT,
        ));
    }
    notes.push((
        texts.format(T::SummaryTarget, &[("path", target.into())]),
        palette::GRAY_1,
    ));
    notes
}

fn finished(ui: &mut Ui, laid: &Laid) {
    for (note, colour) in laid.notes.iter() {
        ui.colored_label(*colour, note);
    }
    list(
        ui,
        (&laid.failed_heading, "failed"),
        &laid.failures,
        Some(&laid.reasons),
        true,
    );
    list(
        ui,
        (&laid.skipped_heading, "skipped"),
        &laid.skipped,
        None,
        false,
    );
}

fn list(
    ui: &mut Ui,
    (heading, salt): (&str, &str),
    lines: &[String],
    reasons: Option<&[usize]>,
    open: bool,
) {
    if lines.is_empty() {
        return;
    }
    ui.add_space(SPACE_2);
    let id = ui.scope_id().with(salt);
    if !section_header(ui, id, heading, open) {
        return;
    }
    ScrollArea::vertical()
        .id_salt(salt)
        .max_height(SECTION_LIST_HEIGHT)
        .show_rows(ui, ROW_HEIGHT, lines.len(), |ui, rows| {
            for index in rows {
                let Some(text) = lines.get(index) else {
                    continue;
                };
                let reason = reasons.and_then(|starts| starts.get(index)).copied();
                list_row(ui, text, reason);
            }
        });
}
