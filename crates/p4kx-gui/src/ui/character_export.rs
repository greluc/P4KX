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
use crate::{
    AssemblyExportText as A, CharacterExportStatus, CharacterExportSummary, CharacterText as T,
    ExtractionText, Model, ModelExportText, Texts, format,
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
            let cancelling = run.is_cancelled();
            let title = texts.get(T::ExportRunningTitle);
            let shown = frames::window(&title, Tone::Normal)
                .id(Id::unique(WINDOW))
                .collapsible(false)
                .resizable(false)
                .default_width(MODAL_WIDTH)
                .show(ui.ctx(), |ui| {
                    frames::head(ui, (&title, Tone::Normal), false, None);
                    cancel = running(ui, texts, written, cancelling);
                });
            decorate(ui, shown, Tone::Normal);
            ui.ctx().request_repaint_after(REFRESH);
        }
        CharacterExportStatus::Finished(summary) => {
            close = outcome_window(
                ui,
                texts,
                (OUTCOME, T::ExportDoneTitle.into(), Tone::Normal),
                |ui| finished(ui, texts, summary),
            );
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

fn running(ui: &mut Ui, texts: &Texts, written: u64, cancelling: bool) -> bool {
    let size = format::size(written, texts.language());
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.label(texts.format(A::Writing, &[("size", size.into())]));
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

fn finished(ui: &mut Ui, texts: &Texts, summary: &CharacterExportSummary) {
    let written = summary.written();
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let counts = [
        ("parts", number(summary.parts())),
        ("bones", number(written.bones)),
        ("vertices", number(written.vertices)),
        ("triangles", number(written.triangles)),
        ("size", format::size(written.bytes, texts.language())),
    ]
    .map(|(name, value)| (name, FluentValue::from(value)));
    let file = [("path", summary.file().display().to_string().into())];
    ui.colored_label(palette::GRAY_1, texts.format(A::SummaryFile, &file));
    ui.colored_label(palette::GRAY_1, texts.format(T::ExportCounts, &counts));
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
