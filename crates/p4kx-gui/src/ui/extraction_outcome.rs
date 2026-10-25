// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::time::Duration;

use egui::text::{LayoutJob, TextFormat};
use egui::{Id, InnerResponse, ScrollArea, Sense, Ui, vec2};
use fluent_bundle::FluentValue;
use p4kx_extraction::Progress;

use super::extraction::number;
use crate::theme::desktop::{
    MODAL_WIDE_WIDTH, MODAL_WIDTH, ROW_HEIGHT, SECTION_LIST_HEIGHT, SPINNER_INLINE,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, SPACE_3, Weight, font};
use crate::widgets::button::Kind;
use crate::widgets::frames::{self, Tone};
use crate::widgets::indicator;
use crate::widgets::placement;
use crate::widgets::rows::{paint_divider, section_header};
use crate::widgets::text::paint_left;
use crate::{
    ChangeExtractionText, ExtractionStatus, ExtractionSummary, ExtractionText as T, Language,
    Model, Texts,
};

const REFRESH: Duration = Duration::from_millis(100);
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
            let title = texts.get(T::RunningTitle);
            let shown = placement::window(ui.ctx(), &title, Tone::Normal)
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
        ExtractionStatus::Finished(summary) => {
            let title = if summary.archive_changed() {
                T::SummaryChangedTitle
            } else if summary.cancelled().is_empty() {
                T::SummaryTitle
            } else {
                T::SummaryCancelledTitle
            };
            let side = extraction.is_comparison_side();
            outcome_window(ui, texts, (title, Tone::Normal), &mut open, |ui| {
                finished(ui, texts, summary, side);
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
            outcome_window(ui, texts, (T::FailedTitle, Tone::Danger), &mut open, |ui| {
                ui.colored_label(palette::DANGER_TEXT, text);
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

pub(super) fn decorate<R>(ui: &Ui, shown: Option<InnerResponse<R>>, tone: Tone) {
    if let Some(shown) = shown {
        frames::modal_decoration(ui.ctx(), shown.response.layer_id, shown.response.rect, tone);
        super::regions::record(ui.ctx(), |regions| {
            regions.dialogs.push(shown.response.rect);
        });
    }
}

fn outcome_window(
    ui: &Ui,
    texts: &Texts,
    (title, tone): (T, Tone),
    open: &mut bool,
    body: impl FnOnce(&mut Ui),
) {
    let mut close = false;
    let title = texts.get(title);
    let close_dialog = texts.get(crate::Text::CloseDialog);
    let shown = placement::window(ui.ctx(), &title, tone)
        .id(Id::unique(WINDOW))
        .collapsible(false)
        .default_width(MODAL_WIDE_WIDTH)
        .show(ui.ctx(), |ui| {
            let closed = frames::head(ui, (&title, tone), false, Some(&close_dialog));
            let room = placement::body_room(ui);
            frames::scrolling_body_in(ui, (WINDOW, room), body);
            close = close_foot(ui, texts) || closed;
        });
    decorate(ui, shown, tone);
    if close {
        *open = false;
    }
}

fn close_foot(ui: &mut Ui, texts: &Texts) -> bool {
    frames::foot_rule(ui);
    frames::foot_action(ui, &texts.get(T::Close), Kind::Cta)
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
    ui.add_space(SPACE_3);
    let width = ui.available_width().max(frames::MODAL_BODY_WIDTH);
    indicator::progress(ui, fraction, cancelling, width);
    frames::foot_rule(ui);
    if cancelling {
        ui.horizontal(|ui| {
            indicator::spinner(ui, SPINNER_INLINE);
            ui.label(texts.get(T::Cancelling));
        });
        false
    } else {
        frames::foot_action(ui, &texts.get(T::Cancel), Kind::Ghost)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a progress bar needs no more than f32 precision"
)]
pub(super) fn ratio(done: u64, total: u64) -> f32 {
    (done as f32 / total as f32).clamp(0.0, 1.0)
}

fn finished(ui: &mut Ui, texts: &Texts, summary: &ExtractionSummary, comparison_side: bool) {
    if summary.archive_changed() {
        let note = if comparison_side {
            texts.get(ChangeExtractionText::SummaryChanged)
        } else {
            texts.get(T::SummaryChanged)
        };
        ui.colored_label(palette::WARNING, note);
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
    if let Some(records) = summary.records() {
        let counts = [
            ("written", number(texts, records.written)),
            ("skipped", number(texts, records.skipped)),
            ("failed", number(texts, records.failed)),
            ("cancelled", number(texts, records.cancelled)),
            ("errors", number(texts, records.error_values)),
            ("erroneous", number(texts, records.records_with_errors)),
        ];
        ui.label(texts.format(T::SummaryRecords, &counts));
    }
    let target = summary.target().display().to_string();
    ui.label(texts.format(T::SummaryTarget, &[("path", target.into())]));
    lists(ui, texts, summary);
}

fn lists(ui: &mut Ui, texts: &Texts, summary: &ExtractionSummary) {
    for (heading, failed) in [
        (T::SummaryFailed, summary.failures()),
        (T::SummaryUnconverted, summary.unconverted()),
    ] {
        let lines = failure_lines(ui, texts, (summary, heading), failed, |failure| {
            let args = [
                ("path", failure.path.clone().into()),
                ("reason", failure.reason.clone().into()),
            ];
            (texts.format(T::SummaryFailure, &args), failure.reason.len())
        });
        list(
            ui,
            texts,
            (heading, &lines.lines, Some(&lines.reasons)),
            true,
        );
    }
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
    list(
        ui,
        texts,
        (T::SummarySkipped, summary.skipped(), None),
        false,
    );
    list(
        ui,
        texts,
        (T::SummaryCancelled, summary.cancelled(), None),
        false,
    );
}

/// The formatted failure lines of a summary and where the reason starts in each.
#[derive(Clone)]
struct FailureLines {
    key: (usize, usize, Language),
    lines: Arc<[String]>,
    reasons: Arc<[usize]>,
}

/// Returns the lines of `failed`, the list under `heading` of `summary`, formatted once per
/// summary and language; `format` returns a line and the length of the reason it ends with.
fn failure_lines<F>(
    ui: &Ui,
    texts: &Texts,
    (summary, heading): (&ExtractionSummary, T),
    failed: &[F],
    format: impl Fn(&F) -> (String, usize),
) -> FailureLines {
    let key = (
        std::ptr::from_ref(summary).addr(),
        failed.len(),
        texts.language(),
    );
    let id = Id::unique("p4kx-failure-lines").with(heading.id());
    if let Some(cached) = ui.data(|data| data.get_temp::<FailureLines>(id))
        && cached.key == key
    {
        return cached;
    }
    let (lines, reasons): (Vec<String>, Vec<usize>) = failed
        .iter()
        .map(|failure| {
            let (line, reason) = format(failure);
            let start = line.len().saturating_sub(reason);
            (line, start)
        })
        .unzip();
    let lines: Arc<[String]> = lines.into();
    let reasons: Arc<[usize]> = reasons.into();
    let failures = FailureLines {
        key,
        lines,
        reasons,
    };
    ui.data_mut(|data| data.insert_temp(id, failures.clone()));
    failures
}

fn list(
    ui: &mut Ui,
    texts: &Texts,
    (heading, lines, reasons): (T, &[String], Option<&[usize]>),
    open: bool,
) {
    if lines.is_empty() {
        return;
    }
    ui.add_space(SPACE_2);
    let title = texts.format(heading, &[("number", number(texts, lines.len()))]);
    let id = ui.scope_id().with(heading.id());
    if !section_header(ui, id, &title, open) {
        return;
    }
    ScrollArea::vertical()
        .id_salt(heading.id())
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

pub(super) fn list_row(ui: &mut Ui, text: &str, reason: Option<usize>) {
    let size = vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, text));
    let inner = rect.shrink2(vec2(SPACE_3, 0.0));
    let (path, cause) = reason
        .and_then(|start| text.split_at_checked(start))
        .unwrap_or((text, ""));
    let mut job = LayoutJob::default();
    let font = font(FS_XS, Weight::Regular);
    for (part, colour) in [(path, palette::GRAY_1), (cause, palette::DANGER_TEXT)] {
        job.append(part, 0.0, TextFormat::simple(font.clone(), colour));
    }
    job.wrap.max_width = inner.width();
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    paint_left(ui, inner, galley, palette::GRAY_1);
    paint_divider(ui.painter(), rect);
}
