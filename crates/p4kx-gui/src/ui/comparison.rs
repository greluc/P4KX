use egui::{Align, Layout, Margin, Panel, Ui, UiBuilder, vec2};
use fluent_bundle::FluentValue;

use super::archive_picker::{self, Picked};
use super::search::eyebrow;
use super::{
    Dialog, RESIZE_GAP, Requests, comparison_diff, comparison_lists, comparison_schema,
    comparison_summary, comparison_table, record_fields,
};
use crate::theme::desktop::{
    COMPARISON_PROGRESS_WIDTH, PREVIEW_MIN_HEIGHT, SPINNER, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{hairline_below, plain};
use crate::widgets::toggle;
use crate::{
    ComparisonSide, ComparisonStage, ComparisonStatus, ComparisonText, Model, RecordChangeText,
    RecordChangeView, Text, Texts, format,
};

const SIDES: [(ComparisonSide, ComparisonText); 2] = [
    (ComparisonSide::Old, ComparisonText::OldArchive),
    (ComparisonSide::New, ComparisonText::NewArchive),
];

/// Shows the Comparison tab: the pickers and levels, the progress, and the result.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let toolbar = frames::panel()
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| setup(ui, model, requests));
    hairline_below(ui, toolbar.response.rect);
    if model.comparison().view().is_some() {
        comparison_summary::show(ui, model);
        comparison_lists::tabs(ui, model);
        let Some(view) = model.comparison().view() else {
            return;
        };
        if comparison_lists::schema_shown(view) {
            frames::panel().show(ui, |ui| comparison_schema::show(ui, model));
            return;
        }
        let records = comparison_lists::records_shown(view);
        let selected = if records {
            view.records().and_then(RecordChangeView::selected)
        } else {
            view.selected()
        };
        let mut list = ui.available_rect_before_wrap();
        if records && selected.is_some() {
            let fields = Panel::bottom("comparison-record-fields")
                .frame(frames::panel().outer_margin(Margin {
                    top: RESIZE_GAP,
                    ..Margin::ZERO
                }))
                .resizable(true)
                .default_size((list.height() / 2.0).floor().max(PREVIEW_MIN_HEIGHT))
                .min_size(PREVIEW_MIN_HEIGHT)
                .show(ui, |ui| record_fields::show(ui, model));
            list.max.y = list.max.y.min(fields.response.rect.top());
        } else if selected.is_some() {
            let diff = Panel::bottom("comparison-diff")
                .frame(frames::panel().outer_margin(Margin {
                    top: RESIZE_GAP,
                    ..Margin::ZERO
                }))
                .resizable(true)
                .default_size((list.height() / 2.0).floor().max(PREVIEW_MIN_HEIGHT))
                .min_size(PREVIEW_MIN_HEIGHT)
                .show(ui, |ui| comparison_diff::show(ui, model));
            list.max.y = list.max.y.min(diff.response.rect.top());
        }
        ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
            frames::panel().show(ui, |ui| comparison_table::show(ui, model, requests));
        });
    } else {
        empty(ui, model.texts());
    }
}

fn setup(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let gap = ui.spacing().item_spacing.x;
    let width = ((ui.available_width() - gap) / 2.0).floor();
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        for (side, text) in SIDES {
            let label = model.texts().get(text);
            let current = model.comparison().archive(side);
            if let Some(choice) = archive_picker::show(ui, model, &label, current, width) {
                picked = Some((side, choice));
            }
        }
    });
    match picked {
        Some((side, Picked::Archive(path))) => model.comparison_mut().set_archive(side, path),
        Some((side, Picked::ChooseFile)) => requests.dialog = Some(Dialog::ComparisonArchive(side)),
        None => {}
    }
    ui.horizontal_wrapped(|ui| {
        levels(ui, model);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            actions(ui, model);
        });
    });
    progress(ui, model);
}

fn levels(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let label = texts.get(ComparisonText::Levels);
    let entries_text = texts.get(ComparisonText::LevelEntries);
    let records_text = texts.get(ComparisonText::LevelRecords);
    let running = model.comparison().is_running();
    let mut levels = model.comparison().levels();
    ui.label(eyebrow(&label));
    toggle::checkbox_if(ui, &mut levels.entries, &entries_text, !running);
    toggle::checkbox_if(ui, &mut levels.records, &records_text, !running);
    if levels != model.comparison().levels() {
        model.comparison_mut().set_levels(levels);
    }
}

fn actions(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    if model.comparison().is_running() {
        let cancel = texts.get(ComparisonText::Cancel);
        if button::button(ui, Kind::Ghost, Size::Normal, &cancel, true).clicked() {
            model.comparison_mut().cancel();
        }
        return;
    }
    let start = texts.get(ComparisonText::Start);
    let enabled = model.comparison().can_start();
    if button::button(ui, Kind::Cta, Size::Normal, &start, enabled).clicked() {
        let ctx = ui.ctx().clone();
        model.comparison_mut().start(move || ctx.request_repaint());
    }
}

fn progress(ui: &mut Ui, model: &Model) {
    let texts = model.texts();
    let (text, colour, fraction) = match model.comparison().status() {
        ComparisonStatus::Running(stage) => {
            let (text, fraction) = stage_text(texts, stage);
            (text, palette::GRAY_1, fraction)
        }
        ComparisonStatus::Cancelled => (
            texts.get(ComparisonText::Cancelled),
            palette::GRAY_2_TEXT,
            None,
        ),
        ComparisonStatus::Failed(Some(reason)) => (
            texts.format(
                ComparisonText::Failed,
                &[("reason", reason.to_owned().into())],
            ),
            palette::DANGER_TEXT,
            None,
        ),
        ComparisonStatus::Failed(None) => (
            texts.get(ComparisonText::Stopped),
            palette::DANGER_TEXT,
            None,
        ),
        ComparisonStatus::Idle | ComparisonStatus::Finished(_) => return,
    };
    ui.horizontal(|ui| {
        if model.comparison().is_running() {
            match fraction {
                Some(fraction) => {
                    let width = ui.available_width().min(COMPARISON_PROGRESS_WIDTH);
                    indicator::progress(ui, fraction, false, width);
                }
                None => indicator::spinner(ui, SPINNER),
            }
        }
        ui.label(plain(text, FS_SM, Weight::Regular).color(colour));
    });
}

fn stage_text(texts: &Texts, stage: ComparisonStage) -> (String, Option<f32>) {
    match stage {
        ComparisonStage::Opening => (texts.get(ComparisonText::Opening), None),
        ComparisonStage::Preparing => (texts.get(ComparisonText::Preparing), None),
        ComparisonStage::Comparing { done, total } => {
            counted(texts, ComparisonText::Progress, (done, total))
        }
        ComparisonStage::ComparingRecords { done, total } => {
            counted(texts, RecordChangeText::Progress, (done, total))
        }
    }
}

fn counted(
    texts: &Texts,
    progress: impl Into<Text>,
    (done, total): (usize, usize),
) -> (String, Option<f32>) {
    let language = texts.language();
    let count = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
    let percent = count(done)
        .saturating_mul(100)
        .checked_div(count(total))
        .unwrap_or(0)
        .min(100);
    let number =
        |value: usize| -> FluentValue<'static> { format::number(count(value), language).into() };
    let text = texts.format(
        progress,
        &[
            ("done", number(done)),
            ("total", number(total)),
            ("percent", percent.to_string().into()),
        ],
    );
    #[expect(
        clippy::cast_precision_loss,
        reason = "a percentage needs no more than f32 precision"
    )]
    let fraction = percent as f32 / 100.0;
    (text, Some(fraction))
}

fn empty(ui: &mut Ui, texts: &Texts) {
    let title = texts.get(ComparisonText::EmptyTitle);
    let hint = texts.get(ComparisonText::EmptyHint);
    super::empty::notice(ui, &title, &hint);
}
