// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::time::Duration;

use egui::{Id, Ui};
use fluent_bundle::FluentValue;

use super::assembly_export_outcome::failed;
use super::extraction_outcome::decorate;
use super::record_export_outcome::outcome_window;
use crate::theme::desktop::{MODAL_WIDTH, SPINNER_INLINE};
use crate::theme::palette;
use crate::theme::tokens::SPACE_3;
use crate::widgets::button::Kind;
use crate::widgets::frames::{self, Tone};
use crate::widgets::indicator;
use crate::widgets::placement;
use crate::{
    AssemblyExportForm, AssemblyExportText as A, CharacterExport, CharacterExportStatus,
    CharacterExportSummary, CharacterText as T, ExtractionText, Model, ModelExportText, Text,
    Texts, format,
};

const REFRESH: Duration = Duration::from_millis(100);
const WINDOW: &str = "character-export-run";
const OUTCOME: &str = "character-export-outcome";

/// Shows the progress of a running Export of a Character as GLB, or its outcome once it ended.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(run) = model.characters().export() else {
        return;
    };
    let texts = model.texts();
    let mut cancel = false;
    let mut close = false;
    match run.status() {
        CharacterExportStatus::Running(written) => {
            cancel = running_window(ui, texts, (run, written));
        }
        CharacterExportStatus::Finished(summary) => {
            let title = if summary.package().is_some() {
                Text::from(A::SummaryTitlePackage)
            } else {
                Text::from(T::ExportDoneTitle)
            };
            close = outcome_window(ui, texts, (OUTCOME, title, Tone::Normal), |ui| {
                finished(ui, texts, summary);
            });
        }
        CharacterExportStatus::Cancelled => {
            let title = (OUTCOME, A::SummaryCancelledTitle.into(), Tone::Normal);
            close = outcome_window(ui, texts, title, |ui| {
                ui.colored_label(palette::WARNING, texts.get(A::SummaryCancelled));
            });
        }
        CharacterExportStatus::Failed(failure) => {
            let name = ("name", FluentValue::from(run.name().to_owned()));
            let (text, colour, tone) = failed(texts, failure, name);
            close = outcome_window(
                ui,
                texts,
                (OUTCOME, ModelExportText::FailedTitle.into(), tone),
                |ui| {
                    ui.colored_label(colour, text);
                },
            );
        }
    }
    if cancel {
        model.characters().cancel_export();
    }
    if close {
        model.characters_mut().close_export();
    }
}

fn running_window(ui: &Ui, texts: &Texts, (run, written): (&CharacterExport, u64)) -> bool {
    let cancelling = run.is_cancelled();
    let package = run.form() == AssemblyExportForm::Package;
    let title = texts.get(if package {
        Text::from(A::RunningTitlePackage)
    } else {
        Text::from(T::ExportRunningTitle)
    });
    let mut cancel = false;
    let shown = placement::window(ui.ctx(), &title, Tone::Normal)
        .id(Id::unique(WINDOW))
        .collapsible(false)
        .resizable(false)
        .default_width(MODAL_WIDTH)
        .show(ui.ctx(), |ui| {
            frames::head(ui, (&title, Tone::Normal), false, None);
            let progress = if package {
                Progress::Package(run.name())
            } else {
                Progress::Bytes(written)
            };
            cancel = running(ui, texts, progress, cancelling);
        });
    decorate(ui, shown, Tone::Normal);
    ui.ctx().request_repaint_after(REFRESH);
    cancel
}

/// What the progress window tells about the running export.
#[derive(Clone, Copy)]
enum Progress<'a> {
    Bytes(u64),
    Package(&'a str),
}

fn running(ui: &mut Ui, texts: &Texts, progress: Progress<'_>, cancelling: bool) -> bool {
    let text = match progress {
        Progress::Bytes(written) => {
            let size = format::size(written, texts.language());
            texts.format(A::Writing, &[("size", size.into())])
        }
        Progress::Package(name) => {
            texts.format(A::WritingPackage, &[("name", name.to_owned().into())])
        }
    };
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.label(text);
    });
    ui.add_space(SPACE_3);
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

/// Returns what the file or the package holds, as a line.
fn counts_line(texts: &Texts, summary: &CharacterExportSummary) -> String {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let args = |counts: &[(&'static str, String)]| {
        counts
            .iter()
            .map(|(name, value)| (*name, FluentValue::from(value.clone())))
            .collect::<Vec<_>>()
    };
    summary.package().map_or_else(
        || {
            let written = summary.written();
            let counts = args(&[
                ("parts", number(summary.parts())),
                ("bones", number(written.bones)),
                ("vertices", number(written.vertices)),
                ("triangles", number(written.triangles)),
                ("size", format::size(written.bytes, texts.language())),
            ]);
            texts.format(T::ExportCounts, &counts)
        },
        |package| {
            let counts = args(&[
                ("parts", number(summary.parts())),
                ("bones", number(package.bones)),
                ("skinned", number(package.skinned_meshes)),
                ("morphs", number(package.morph_targets)),
                ("animations", number(package.animations())),
                ("vertices", number(package.vertices)),
                ("triangles", number(package.triangles)),
                ("files", number(package.files)),
                ("size", format::size(package.bytes, texts.language())),
            ]);
            texts.format(T::ExportPackageCounts, &counts)
        },
    )
}

fn finished(ui: &mut Ui, texts: &Texts, summary: &CharacterExportSummary) {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let file = [("path", summary.file().display().to_string().into())];
    ui.colored_label(palette::GRAY_1, texts.format(A::SummaryFile, &file));
    ui.colored_label(palette::GRAY_1, counts_line(texts, summary));
    if summary.skipped() > 0 {
        let skipped = [
            (
                "count",
                u64::try_from(summary.skipped()).unwrap_or(u64::MAX).into(),
            ),
            ("number", number(summary.skipped()).into()),
        ];
        ui.colored_label(palette::WARNING, texts.format(T::ExportSkipped, &skipped));
    }
}
