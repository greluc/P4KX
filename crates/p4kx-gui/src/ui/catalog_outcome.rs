use std::sync::Arc;
use std::time::Duration;

use egui::{Id, Ui};
use fluent_bundle::FluentValue;
use p4kx_export::{CatalogSummary, DECOMPRESSED_LIMIT, SizeWarning, UPLOAD_LIMIT};

use super::extraction::{group_label, number};
use super::extraction_outcome::{decorate, ratio};
use super::record_export_outcome::{list, outcome_window};
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
            let laid = Laid::get(ui, texts, (run.id(), summary));
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
        CatalogExportStatus::IsArchive => {
            let text = texts.format(T::IsArchive, &[("path", run.shown_file().into())]);
            close = failure_window(ui, texts, text);
        }
        CatalogExportStatus::Failed(reason) => {
            let path = ("path", run.shown_file().into());
            let text = match reason {
                Some(reason) => texts.format(T::Failed, &[path, ("reason", reason.into())]),
                None => texts.format(T::Stopped, &[path]),
            };
            close = failure_window(ui, texts, text);
        }
    }
    if cancel {
        model.cancel_catalog();
    }
    if close {
        model.close_catalog();
    }
}

/// Shows the danger window of a catalog that wrote nothing, saying why; returns whether it was
/// closed.
fn failure_window(ui: &Ui, texts: &Texts, text: String) -> bool {
    outcome_window(
        ui,
        texts,
        (OUTCOME, T::FailedTitle.into(), Tone::Danger),
        |ui| {
            ui.colored_label(palette::DANGER_TEXT, text);
        },
    )
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
    let path = run.shown_file();
    let shown = frames::window(&title, Tone::Normal)
        .id(Id::unique(WINDOW))
        .collapsible(false)
        .resizable(false)
        .default_width(MODAL_WIDTH)
        .show(ui.ctx(), |ui| {
            frames::head(ui, (&title, Tone::Normal), false, None);
            cancel = running(ui, texts, (progress, path), cancelling);
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
        CatalogExportStage::Writing => {
            waiting(ui, &texts.format(T::Writing, &[("path", path.into())]));
        }
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

/// The texts of one summary, formatted once per catalog and language.
struct Laid {
    key: (u64, Language),
    headings: [String; 2],
    warnings: Vec<(String, SizeWarning)>,
    kinds: Vec<String>,
    skipped: String,
    failed_heading: String,
    failures: Vec<String>,
    reasons: Vec<usize>,
    texts: Vec<String>,
    missing: String,
    size: String,
    file: String,
}

impl Laid {
    fn get(ui: &Ui, texts: &Texts, (run, summary): (u64, &CatalogExportSummary)) -> Arc<Self> {
        let key = (run, texts.language());
        let id = Id::unique("p4kx-catalog-summary");
        if let Some(cached) = ui.data(|data| data.get_temp::<Arc<Self>>(id))
            && cached.key == key
        {
            return cached;
        }
        let laid = Arc::new(Self::new(key, texts, summary));
        ui.data_mut(|data| data.insert_temp(id, Arc::clone(&laid)));
        laid
    }

    fn new(key: (u64, Language), texts: &Texts, outcome: &CatalogExportSummary) -> Self {
        let summary = outcome.summary();
        let limit = format::decimal_size(UPLOAD_LIMIT, texts.language());
        let decompressed = format::decimal_size(DECOMPRESSED_LIMIT, texts.language());
        let warnings = [
            (summary.size_warning(), [T::NearLimit, T::OverLimit], &limit),
            (
                summary.decompressed_warning(),
                [T::DecompressedNearLimit, T::DecompressedOverLimit],
                &decompressed,
            ),
        ]
        .into_iter()
        .filter_map(|(warning, [near, over], shown)| {
            let warning = warning?;
            let text = match warning {
                SizeWarning::NearLimit => near,
                SizeWarning::OverLimit => over,
            };
            Some((
                texts.format(text, &[("limit", shown.clone().into())]),
                warning,
            ))
        })
        .collect();
        let (failures, reasons) = failure_lines(texts, summary);
        Self {
            key,
            headings: [texts.get(T::KindsHeading), texts.get(T::TextsHeading)],
            warnings,
            kinds: kinds(texts, summary),
            skipped: texts.format(
                T::SummarySkipped,
                &[("number", number(texts, summary.skipped()))],
            ),
            failed_heading: texts.format(
                T::SummaryFailed,
                &[("number", number(texts, summary.failures().len()))],
            ),
            failures,
            reasons,
            texts: languages(texts, summary),
            missing: texts.format(
                T::SummaryMissing,
                &[("number", number(texts, summary.unresolved()))],
            ),
            size: sizes(texts, summary, limit),
            file: texts.format(
                T::SummaryFile,
                &[("path", outcome.file().display().to_string().into())],
            ),
        }
    }
}

/// Returns the line with the size of the catalog file against the upload limit, and the
/// uncompressed size beside it for a compressed catalog.
fn sizes(texts: &Texts, summary: &CatalogSummary, limit: String) -> String {
    let size = format::decimal_size(summary.size(), texts.language());
    match summary.compressed_size() {
        Some(compressed) => texts.format(
            T::SummarySizeCompressed,
            &[
                (
                    "compressed",
                    format::decimal_size(compressed, texts.language()).into(),
                ),
                ("size", size.into()),
                ("limit", limit.into()),
            ],
        ),
        None => texts.format(
            T::SummarySize,
            &[("size", size.into()), ("limit", limit.into())],
        ),
    }
}

/// Returns the line of every Record that could not be read and where its reason starts.
fn failure_lines(texts: &Texts, summary: &CatalogSummary) -> (Vec<String>, Vec<usize>) {
    summary
        .failures()
        .iter()
        .map(|failure| {
            let line = texts.format(
                T::SummaryFailure,
                &[
                    ("path", failure.record_path.clone().into()),
                    ("guid", failure.guid.to_string().into()),
                    ("reason", failure.reason.clone().into()),
                ],
            );
            let reason = line.len().saturating_sub(failure.reason.len());
            (line, reason)
        })
        .unzip()
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
    for (warning, level) in &laid.warnings {
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
    list(
        ui,
        (&laid.failed_heading, "catalog-failed"),
        &laid.failures,
        Some(&laid.reasons),
        true,
    );
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
