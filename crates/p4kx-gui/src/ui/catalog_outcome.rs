use std::sync::Arc;
use std::time::Duration;

use egui::{Id, Ui};
use fluent_bundle::FluentValue;
use p4kx_export::{CatalogSummary, SizeWarning, UPLOAD_LIMIT};

use super::extraction::{group_label, number};
use super::extraction_outcome::{decorate, ratio};
use super::record_export_outcome::outcome_window;
use crate::theme::desktop::{MODAL_GROUP_GAP, MODAL_WIDTH, SPINNER_INLINE};
use crate::theme::palette;
use crate::theme::tokens::SPACE_3;
use crate::widgets::button::Kind;
use crate::widgets::frames::{self, Tone};
use crate::widgets::indicator;
use crate::{
    CatalogExport, CatalogExportProgress, CatalogExportStage, CatalogExportStatus,
    CatalogExportSummary, CatalogText as T, ExtractionText, Language, Model, Texts, format,
};

const REFRESH: Duration = Duration::from_millis(100);
const WINDOW: &str = "catalog-run";
const OUTCOME: &str = "catalog-outcome";

/// Shows the progress of a running Basetool catalog, or its outcome once it ended.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(run) = model.catalog() else {
        return;
    };
    let texts = model.texts();
    let mut cancel = false;
    let mut close = false;
    match run.status() {
        CatalogExportStatus::Running(progress) => {
            cancel = running_window(ui, texts, run, progress);
        }
        CatalogExportStatus::Finished(summary) => {
            let laid = Laid::get(ui, texts, summary);
            close = outcome_window(
                ui,
                texts,
                (OUTCOME, T::SummaryTitle.into(), Tone::Normal),
                |ui| finished(ui, &laid),
            );
        }
        CatalogExportStatus::Cancelled => {
            close = outcome_window(
                ui,
                texts,
                (OUTCOME, T::CancelledTitle.into(), Tone::Normal),
                |ui| {
                    ui.colored_label(palette::WARNING, texts.get(T::Cancelled));
                },
            );
        }
        CatalogExportStatus::Failed(reason) => {
            let path = ("path", run.file().display().to_string().into());
            let text = match reason {
                Some(reason) => {
                    texts.format(T::Failed, &[path, ("reason", reason.to_owned().into())])
                }
                None => texts.format(T::Stopped, &[path]),
            };
            close = outcome_window(
                ui,
                texts,
                (OUTCOME, T::FailedTitle.into(), Tone::Danger),
                |ui| {
                    ui.colored_label(palette::DANGER_TEXT, text);
                },
            );
        }
    }
    if cancel {
        model.cancel_catalog();
    }
    if close {
        model.close_catalog();
    }
}

/// Shows the non-modal progress window; returns whether Cancel was pressed.
fn running_window(
    ui: &Ui,
    texts: &Texts,
    run: &CatalogExport,
    progress: CatalogExportProgress,
) -> bool {
    let mut cancel = false;
    let cancelling = run.is_cancelled();
    let title = texts.get(T::RunningTitle);
    let path = run.file().display().to_string();
    let shown = frames::window(&title, Tone::Normal)
        .id(Id::unique(WINDOW))
        .collapsible(false)
        .resizable(false)
        .default_width(MODAL_WIDTH)
        .show(ui.ctx(), |ui| {
            frames::head(ui, (&title, Tone::Normal), false, None);
            cancel = running(ui, texts, (progress, &path), cancelling);
        });
    decorate(ui, shown, Tone::Normal);
    ui.ctx().request_repaint_after(REFRESH);
    cancel
}

fn running(
    ui: &mut Ui,
    texts: &Texts,
    (progress, path): (CatalogExportProgress, &str),
    cancelling: bool,
) -> bool {
    let fraction = match progress.stage {
        CatalogExportStage::Building if progress.total > 0 => {
            ratio(progress.done as u64, progress.total as u64)
        }
        CatalogExportStage::Writing => 1.0,
        _ => 0.0,
    };
    match progress.stage {
        CatalogExportStage::Reading => waiting(ui, &texts.get(T::Reading)),
        CatalogExportStage::Writing => waiting(
            ui,
            &texts.format(T::Writing, &[("path", path.to_owned().into())]),
        ),
        CatalogExportStage::Building => {
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
        waiting(ui, &texts.get(ExtractionText::Cancelling));
        false
    } else {
        frames::foot_action(ui, &texts.get(ExtractionText::Cancel), Kind::Ghost)
    }
}

fn waiting(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.label(text);
    });
}

/// The texts of one summary, formatted once per summary and language.
#[derive(Clone)]
struct Laid {
    key: (usize, Language),
    headings: [String; 2],
    warning: Option<(String, SizeWarning)>,
    kinds: Arc<[String]>,
    skipped: String,
    texts: Arc<[String]>,
    missing: String,
    size: String,
    file: String,
}

impl Laid {
    fn get(ui: &Ui, texts: &Texts, summary: &CatalogExportSummary) -> Self {
        let key = (std::ptr::from_ref(summary).addr(), texts.language());
        let id = Id::unique("p4kx-catalog-summary");
        if let Some(cached) = ui.data(|data| data.get_temp::<Self>(id))
            && cached.key == key
        {
            return cached;
        }
        let laid = Self::new(key, texts, summary);
        ui.data_mut(|data| data.insert_temp(id, laid.clone()));
        laid
    }

    fn new(key: (usize, Language), texts: &Texts, outcome: &CatalogExportSummary) -> Self {
        let summary = outcome.summary();
        let limit = format::size(UPLOAD_LIMIT, texts.language());
        let warning = summary.size_warning().map(|warning| {
            let text = match warning {
                SizeWarning::NearLimit => T::NearLimit,
                SizeWarning::OverLimit => T::OverLimit,
            };
            (
                texts.format(text, &[("limit", limit.clone().into())]),
                warning,
            )
        });
        Self {
            key,
            headings: [texts.get(T::KindsHeading), texts.get(T::TextsHeading)],
            warning,
            kinds: kinds(texts, summary).into(),
            skipped: texts.format(
                T::SummarySkipped,
                &[("number", number(texts, summary.skipped()))],
            ),
            texts: languages(texts, summary).into(),
            missing: texts.format(
                T::SummaryMissing,
                &[("number", number(texts, summary.unresolved()))],
            ),
            size: texts.format(
                T::SummarySize,
                &[
                    (
                        "size",
                        format::size(summary.size(), texts.language()).into(),
                    ),
                    ("limit", limit.into()),
                ],
            ),
            file: texts.format(
                T::SummaryFile,
                &[("path", outcome.file().display().to_string().into())],
            ),
        }
    }
}

fn kinds(texts: &Texts, summary: &CatalogSummary) -> Vec<String> {
    summary
        .kinds()
        .iter()
        .map(|kind| {
            texts.format(
                T::SummaryKind,
                &[
                    ("kind", kind.kind.name().into()),
                    ("written", number(texts, kind.count)),
                    ("skipped", number(texts, kind.skipped)),
                    ("complete", if kind.complete { "yes" } else { "no" }.into()),
                ],
            )
        })
        .collect()
}

fn languages(texts: &Texts, summary: &CatalogSummary) -> Vec<String> {
    summary
        .texts()
        .iter()
        .map(|text| {
            let language = ("language", text.language.code().into());
            let missing = ("missing", number(texts, text.unresolved));
            if text.available {
                texts.format(
                    T::SummaryTexts,
                    &[
                        language,
                        ("resolved", number(texts, text.resolved)),
                        missing,
                        ("placeholders", number(texts, text.placeholder)),
                    ],
                )
            } else {
                texts.format(T::SummaryTextsUnavailable, &[language, missing])
            }
        })
        .collect()
}

fn finished(ui: &mut Ui, laid: &Laid) {
    if let Some((warning, level)) = &laid.warning {
        let body = |ui: &mut Ui| {
            ui.colored_label(palette::GRAY_1, warning);
        };
        match level {
            SizeWarning::NearLimit => frames::warning_alert(ui, body),
            SizeWarning::OverLimit => frames::danger_alert(ui, body),
        }
        ui.add_space(MODAL_GROUP_GAP);
    }
    group_label(ui, &laid.headings[0]);
    lines(ui, &laid.kinds);
    note(ui, &laid.skipped);
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &laid.headings[1]);
    lines(ui, &laid.texts);
    note(ui, &laid.missing);
    ui.add_space(MODAL_GROUP_GAP);
    note(ui, &laid.size);
    note(ui, &laid.file);
}

fn lines(ui: &mut Ui, lines: &[String]) {
    for line in lines {
        note(ui, line);
    }
}

fn note(ui: &mut Ui, text: &str) {
    ui.colored_label(palette::GRAY_1, text);
}
