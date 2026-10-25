// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Color32, Label, ScrollArea, Ui, vec2};
use fluent_bundle::FluentValue;

use super::search::eyebrow;
use super::{Dialog, Requests};
use crate::theme::desktop::{SEGMENT_PADDING_X, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::indicator;
use crate::widgets::tab;
use crate::widgets::text::{line, plain};
use crate::{
    Model, Texts, UiBackground, UiExport, UiExportStatus, UiFormat, UiMode, UiSettings, UiSize,
    UiText,
};

/// A labelled group of segments.
struct Cluster {
    label: UiText,
    items: Vec<UiText>,
    active: usize,
}

impl Cluster {
    fn of<T: Copy + PartialEq>(
        label: UiText,
        (all, current): (&[T], T),
        text: impl Fn(T) -> UiText,
    ) -> Self {
        Self {
            label,
            items: all.iter().map(|item| text(*item)).collect(),
            active: all.iter().position(|item| *item == current).unwrap_or(0),
        }
    }

    fn width(&self, ui: &Ui, texts: &Texts) -> f32 {
        let label = line(ui, eyebrow(&texts.get(self.label)), f32::INFINITY)
            .size()
            .x;
        let segments: f32 = self
            .items
            .iter()
            .map(|item| {
                let name = plain(texts.get(*item), FS_XS, Weight::Bold);
                2.0f32.mul_add(SEGMENT_PADDING_X, line(ui, name, f32::INFINITY).size().x)
            })
            .sum();
        label + TOOLBAR_PADDING + segments
    }

    fn show(&self, ui: &mut Ui, texts: &Texts) -> Option<usize> {
        let names: Vec<String> = self.items.iter().map(|item| texts.get(*item)).collect();
        ui.label(eyebrow(&texts.get(self.label)));
        let clicked = tab::segments(ui, &names, Some(self.active));
        ui.add_space(TOOLBAR_PADDING);
        clicked
    }
}

/// What the toolbar was used for.
#[derive(Default)]
struct Chosen {
    mode: Option<UiMode>,
    size: Option<UiSize>,
    background: Option<UiBackground>,
    format: Option<UiFormat>,
    export: bool,
}

/// Shows the choice of the source, of the size, background and format, and the export, then the
/// state of the export.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let width = ui.available_width();
    let chosen = ScrollArea::horizontal()
        .id_salt("ui-toolbar")
        .auto_shrink([false, true])
        .show(ui, |ui| choices(ui, model, width))
        .inner;
    if let Some(mode) = chosen.mode {
        model.ui_mut().set_mode(mode);
    }
    if let Some(size) = chosen.size {
        model.ui_mut().set_size(size);
    }
    if let Some(background) = chosen.background {
        model.ui_mut().set_background(background);
    }
    if let Some(format) = chosen.format {
        model.ui_mut().set_format(format);
    }
    if chosen.export {
        requests.dialog = Some(Dialog::UiFile);
    }
    outcome(ui, model);
}

fn choices(ui: &mut Ui, model: &Model, width: f32) -> Chosen {
    let texts = model.texts();
    let tab = model.ui();
    let UiSettings {
        size,
        background,
        format,
    } = *tab.settings();
    let clusters = [
        Cluster::of(
            UiText::ModeLabel,
            (&[UiMode::Files, UiMode::Ships], tab.mode()),
            |mode| match mode {
                UiMode::Files => UiText::ModeFiles,
                UiMode::Ships => UiText::ModeShips,
            },
        ),
        Cluster::of(UiText::SizeLabel, (&UiSize::ALL, size), UiSize::text),
        Cluster::of(
            UiText::BackgroundLabel,
            (&UiBackground::ALL, background),
            UiBackground::text,
        ),
        Cluster::of(
            UiText::FormatLabel,
            (&UiFormat::ALL, format),
            UiFormat::text,
        ),
    ];
    let export = texts.get(UiText::Export);
    let button_width = button::width(ui, Size::Normal, &export);
    let mut widths: Vec<f32> = clusters.iter().map(|c| c.width(ui, texts)).collect();
    widths.push(button_width);
    let rows = pack(&widths, width, TOOLBAR_PADDING);
    let mut chosen = Chosen::default();
    let available = tab.export_available();
    for row in rows {
        ui.horizontal(|ui| {
            for position in row {
                let Some(cluster) = clusters.get(position) else {
                    chosen.export =
                        button::button(ui, Kind::Cta, Size::Normal, &export, available).clicked();
                    continue;
                };
                let picked = cluster.show(ui, texts);
                match position {
                    0 => chosen.mode = picked.and_then(mode_at),
                    1 => chosen.size = picked.and_then(|at| UiSize::ALL.get(at).copied()),
                    2 => {
                        chosen.background =
                            picked.and_then(|at| UiBackground::ALL.get(at).copied());
                    }
                    _ => chosen.format = picked.and_then(|at| UiFormat::ALL.get(at).copied()),
                }
            }
        });
    }
    chosen
}

fn mode_at(at: usize) -> Option<UiMode> {
    [UiMode::Files, UiMode::Ships].get(at).copied()
}

/// Splits items of the given widths into rows that each fit `available`, in order.
fn pack(widths: &[f32], available: f32, gap: f32) -> Vec<std::ops::Range<usize>> {
    let mut rows = Vec::new();
    let (mut start, mut used) = (0, 0.0_f32);
    for (position, width) in widths.iter().enumerate() {
        let needed = if position == start {
            *width
        } else {
            used + gap + width
        };
        if position > start && needed > available {
            rows.push(start..position);
            (start, used) = (position, *width);
        } else {
            used = needed;
        }
    }
    rows.push(start..widths.len());
    rows
}

fn with_file<'a>(
    file: &str,
    extra: &[(&'a str, FluentValue<'a>)],
) -> Vec<(&'a str, FluentValue<'a>)> {
    let mut all = vec![("file", FluentValue::from(file.to_owned()))];
    all.extend(extra.iter().cloned());
    all
}

fn outcome(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let Some(export) = model.ui().export() else {
        return;
    };
    let (text, tone, running) = status_line(texts, export);
    let dismiss = texts.get(UiText::ExportDismiss);
    let mut dismissed = false;
    ui.horizontal_wrapped(|ui| {
        if running {
            indicator::spinner(ui, SPINNER_INLINE);
        }
        ui.add(Label::new(plain(text, FS_SM, Weight::Regular).color(tone)).selectable(true));
        if !running {
            dismissed = button::button(ui, Kind::Ghost, Size::Compact, &dismiss, true).clicked();
        }
    });
    if dismissed {
        model.ui_mut().dismiss_export();
    }
}

fn status_line(texts: &Texts, export: &UiExport) -> (String, Color32, bool) {
    let file = export.file().display().to_string();
    match export.status() {
        UiExportStatus::Running => (
            texts.format(UiText::ExportRunning, &with_file(&file, &[])),
            palette::GRAY_1,
            true,
        ),
        UiExportStatus::Done(summary) => {
            let size = [
                ("width", FluentValue::from(u64::from(summary.width))),
                ("height", FluentValue::from(u64::from(summary.height))),
            ];
            if summary.left_out == 0 {
                (
                    texts.format(UiText::ExportDone, &with_file(&file, &size)),
                    palette::SUCCESS_TEXT,
                    false,
                )
            } else {
                let count = u64::try_from(summary.left_out).unwrap_or(u64::MAX);
                let mut all = with_file(&file, &size);
                all.push(("count", FluentValue::from(count)));
                all.push(("number", FluentValue::from(count.to_string())));
                (
                    texts.format(UiText::ExportDoneIncomplete, &all),
                    palette::WARNING,
                    false,
                )
            }
        }
        UiExportStatus::IsArchive => (
            texts.get(UiText::ExportIsArchive),
            palette::DANGER_TEXT,
            false,
        ),
        UiExportStatus::ArchiveChanged => (
            texts.get(UiText::ExportArchiveChanged),
            palette::WARNING,
            false,
        ),
        UiExportStatus::Failed(Some(reason)) => (
            texts.format(
                UiText::ExportFailed,
                &[("reason", reason.to_owned().into())],
            ),
            palette::DANGER_TEXT,
            false,
        ),
        UiExportStatus::Failed(None) => (
            texts.get(UiText::ExportStopped),
            palette::DANGER_TEXT,
            false,
        ),
    }
}
