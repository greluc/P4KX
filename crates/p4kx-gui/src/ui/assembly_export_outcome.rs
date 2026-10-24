// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Id, Ui};
use fluent_bundle::FluentValue;

use super::extraction_outcome::decorate;
use super::record_export_outcome::{list, outcome_window};
use crate::model_info::printable;
use crate::theme::desktop::{MODAL_WIDTH, SPINNER_INLINE};
use crate::theme::palette;
use crate::theme::tokens::SPACE_3;
use crate::widgets::button::Kind;
use crate::widgets::frames::{self, Tone};
use crate::widgets::indicator;
use crate::{
    AssemblyExportProgress, AssemblyExportStage, AssemblyExportStatus, AssemblyExportSummary,
    AssemblyExportText as T, ExtractionText, Language, Model, ModelExportFailure, ModelExportText,
    Texts, format,
};

const REFRESH: Duration = Duration::from_millis(100);
const WINDOW: &str = "assembly-export-run";
const OUTCOME: &str = "assembly-export-outcome";

/// Shows the progress of a running Export of an Entity as GLB, or its outcome once it ended.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(run) = model.assembly_export() else {
        return;
    };
    let texts = model.texts();
    let name = || ("name", run.name().to_owned().into());
    let mut cancel = false;
    let mut close = false;
    match run.status() {
        AssemblyExportStatus::Running(progress) => {
            let cancelling = run.is_cancelled();
            let title = texts.get(T::RunningTitle);
            let shown = frames::window(&title, Tone::Normal)
                .id(Id::unique(WINDOW))
                .collapsible(false)
                .resizable(false)
                .default_width(MODAL_WIDTH)
                .show(ui.ctx(), |ui| {
                    frames::head(ui, (&title, Tone::Normal), false, None);
                    cancel = running(ui, texts, (run.name(), progress), cancelling);
                });
            decorate(ui, shown, Tone::Normal);
            ui.ctx().request_repaint_after(REFRESH);
        }
        AssemblyExportStatus::Finished(summary) => {
            let laid = Laid::get(ui, texts, (run.id(), summary));
            close = outcome_window(
                ui,
                texts,
                (OUTCOME, T::SummaryTitle.into(), Tone::Normal),
                |ui| finished(ui, &laid),
            );
        }
        AssemblyExportStatus::Cancelled => {
            let title = (OUTCOME, T::SummaryCancelledTitle.into(), Tone::Normal);
            close = outcome_window(ui, texts, title, |ui| {
                ui.colored_label(palette::WARNING, texts.get(T::SummaryCancelled));
            });
        }
        AssemblyExportStatus::Failed(failure) => {
            let (text, colour, tone) = failed(texts, failure, name());
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
        model.cancel_assembly_export();
    }
    if close {
        model.close_assembly_export();
    }
}

fn failed(
    texts: &Texts,
    failure: &ModelExportFailure,
    name: (&str, FluentValue<'static>),
) -> (String, Color32, Tone) {
    use ModelExportText as M;
    match failure {
        ModelExportFailure::KeyRequired => (
            texts.format(M::KeyRequired, &[name]),
            palette::WARNING,
            Tone::Normal,
        ),
        ModelExportFailure::KeyRejected => (
            texts.format(M::KeyRejected, &[name]),
            palette::DANGER_TEXT,
            Tone::Danger,
        ),
        ModelExportFailure::ArchiveChanged => (
            texts.format(M::ArchiveChanged, &[name]),
            palette::WARNING,
            Tone::Normal,
        ),
        ModelExportFailure::Refused(refused) => (
            texts.format(
                ExtractionText::TargetInInstallation,
                &[(
                    "installation",
                    refused.installation.display().to_string().into(),
                )],
            ),
            palette::DANGER_TEXT,
            Tone::Danger,
        ),
        ModelExportFailure::Failed(reason) => (
            texts.format(M::Failed, &[name, ("reason", reason.clone().into())]),
            palette::DANGER_TEXT,
            Tone::Danger,
        ),
        ModelExportFailure::Stopped => (
            texts.format(M::Stopped, &[name]),
            palette::DANGER_TEXT,
            Tone::Danger,
        ),
    }
}

fn running(
    ui: &mut Ui,
    texts: &Texts,
    (name, progress): (&str, AssemblyExportProgress),
    cancelling: bool,
) -> bool {
    let name = ("name", FluentValue::from(name.to_owned()));
    let text = match progress.stage {
        AssemblyExportStage::Resolving => texts.format(T::Resolving, &[name]),
        AssemblyExportStage::Reading => texts.format(T::Reading, &[name]),
        AssemblyExportStage::Writing => texts.format(
            T::Writing,
            &[("size", format::size(progress.done, texts.language()).into())],
        ),
    };
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.label(text);
    });
    ui.add_space(SPACE_3);
    let width = ui.available_width().max(frames::MODAL_BODY_WIDTH);
    indicator::progress(ui, stage_fraction(progress.stage), cancelling, width);
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

/// The share of the export each step stands for: the models are the longest step.
const fn stage_fraction(stage: AssemblyExportStage) -> f32 {
    match stage {
        AssemblyExportStage::Resolving => 0.1,
        AssemblyExportStage::Reading => 0.4,
        AssemblyExportStage::Writing => 0.8,
    }
}

/// The texts of one summary, formatted once per export and language.
struct Laid {
    key: (u64, Language),
    notes: Vec<(String, Color32)>,
    heading: String,
    issues: Vec<String>,
    textures_heading: String,
    textures: Vec<String>,
}

impl Laid {
    fn get(ui: &Ui, texts: &Texts, (run, summary): (u64, &AssemblyExportSummary)) -> Arc<Self> {
        let key = (run, texts.language());
        let id = Id::unique("p4kx-assembly-export-summary");
        if let Some(cached) = ui.data(|data| data.get_temp::<Arc<Self>>(id))
            && cached.key == key
        {
            return cached;
        }
        let laid = Arc::new(Self::new(key, texts, summary));
        ui.data_mut(|data| data.insert_temp(id, Arc::clone(&laid)));
        laid
    }

    fn new(key: (u64, Language), texts: &Texts, summary: &AssemblyExportSummary) -> Self {
        let (textures_heading, textures) = missing_textures(texts, summary);
        Self {
            key,
            textures_heading,
            textures,
            notes: notes(texts, summary),
            heading: texts.format(
                T::ReportHeading,
                &[("number", number(texts, summary.issues().len()).into())],
            ),
            issues: summary
                .issues()
                .iter()
                .map(|issue| {
                    let path = if issue.path.is_empty() {
                        issue.name.clone()
                    } else {
                        issue.path.clone()
                    };
                    let args = [
                        ("path", path.into()),
                        ("name", issue.name.clone().into()),
                        ("reason", issue.reason.clone().into()),
                    ];
                    texts.format(
                        if issue.skipped {
                            T::ReportSkipped
                        } else {
                            T::ReportNote
                        },
                        &args,
                    )
                })
                .collect(),
        }
    }
}

/// Returns the heading and the lines of the textures the file lacks.
fn missing_textures(texts: &Texts, summary: &AssemblyExportSummary) -> (String, Vec<String>) {
    let fallbacks = &summary.written().texture_fallbacks;
    let heading = texts.format(
        ModelExportText::SummaryTextures,
        &[("number", number(texts, fallbacks.len()).into())],
    );
    let lines = fallbacks
        .iter()
        .map(|texture| {
            texts.format(
                ModelExportText::SummaryTexture,
                &[
                    ("material", printable(&texture.material).into()),
                    ("role", texture.role.name().into()),
                    ("file", printable(&texture.file).into()),
                    ("reason", printable(&texture.reason).into()),
                ],
            )
        })
        .collect();
    (heading, lines)
}

fn number(texts: &Texts, value: usize) -> String {
    format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language())
}

/// Returns the lines above the report, each with its colour.
fn notes(texts: &Texts, summary: &AssemblyExportSummary) -> Vec<(String, Color32)> {
    let written = summary.written();
    let counts = [
        ("parts", number(texts, summary.parts())),
        ("modelled", number(texts, summary.modelled_parts())),
        ("nodes", number(texts, written.nodes)),
        ("meshes", number(texts, written.meshes)),
        ("materials", number(texts, written.materials)),
        ("vertices", number(texts, written.vertices)),
        ("triangles", number(texts, written.triangles)),
        ("size", format::size(written.bytes, texts.language())),
    ]
    .map(|(name, value)| (name, FluentValue::from(value)));
    let mut notes = vec![
        (
            texts.format(
                T::SummaryFile,
                &[("path", summary.file().display().to_string().into())],
            ),
            palette::GRAY_1,
        ),
        (texts.format(T::SummaryCounts, &counts), palette::GRAY_1),
    ];
    if written.textures > 0 || !written.texture_fallbacks.is_empty() {
        let args = [
            (
                "images",
                u64::try_from(written.textures).unwrap_or(u64::MAX).into(),
            ),
            ("textured", number(texts, written.textured_materials).into()),
            ("materials", number(texts, written.materials).into()),
        ];
        notes.push((texts.format(T::SummaryImages, &args), palette::GRAY_1));
    }
    for (text, value) in [
        (T::SummaryFallbacks, written.lod_fallbacks),
        (T::SummaryHidden, written.hidden_primitives),
    ] {
        if value > 0 {
            let args = [
                ("count", u64::try_from(value).unwrap_or(u64::MAX).into()),
                ("number", number(texts, value).into()),
            ];
            notes.push((texts.format(text, &args), palette::WARNING));
        }
    }
    if summary.issues().is_empty() {
        notes.push((texts.get(T::ReportAll), palette::GRAY_1));
    }
    notes
}

fn finished(ui: &mut Ui, laid: &Laid) {
    for (note, colour) in &laid.notes {
        ui.colored_label(*colour, note);
    }
    list(
        ui,
        (&laid.textures_heading, "assembly-export-textures"),
        &laid.textures,
        None,
        true,
    );
    list(
        ui,
        (&laid.heading, "assembly-export-report"),
        &laid.issues,
        None,
        true,
    );
}
