use egui::{
    Align, CornerRadius, Frame, Layout, Margin, Panel, Role, Ui, UiBuilder, WidgetInfo, vec2,
};
use fluent_bundle::FluentValue;

use super::archive_picker::{self, Picked};
use super::search::eyebrow;
use super::{Dialog, RESIZE_GAP, Requests, comparison_diff, comparison_summary, comparison_table};
use crate::theme::desktop::{
    COMPARISON_PROGRESS_WIDTH, EMPTY_ICON, EMPTY_PADDING, PREVIEW_MIN_HEIGHT, SPINNER, TITLE_SIZE,
    TITLE_TRACKING, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, SPACE_3, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{caps, hairline_below, plain};
use crate::widgets::toggle;
use crate::{
    ComparisonSide, ComparisonStage, ComparisonStatus, ComparisonText, Model, Texts, format,
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
    if let Some(view) = model.comparison().view() {
        comparison_summary::show(ui, model);
        let mut list = ui.available_rect_before_wrap();
        if view.selected().is_some() {
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
            let language = texts.language();
            let count = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
            let percent = count(done)
                .saturating_mul(100)
                .checked_div(count(total))
                .unwrap_or(0)
                .min(100);
            let number = |value: usize| -> FluentValue<'static> {
                format::number(count(value), language).into()
            };
            let text = texts.format(
                ComparisonText::Progress,
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
    }
}

fn empty(ui: &mut Ui, texts: &Texts) {
    let rect = ui.available_rect_before_wrap();
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette::BG_DARK_GRAY);
    frames::dashed(ui, rect.shrink(0.5), palette::GRAY_3);
    Frame::NONE
        .inner_margin(Margin::same(px(EMPTY_PADDING)))
        .show(ui, |ui| {
            ui.set_min_size(rect.size() - vec2(2.0 * EMPTY_PADDING, 2.0 * EMPTY_PADDING));
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.add_space((rect.height() / 2.0 - EMPTY_ICON).max(0.0));
                let title = texts.get(ComparisonText::EmptyTitle);
                ui.label(
                    caps(&title, TITLE_SIZE, Weight::Bold, TITLE_TRACKING).color(palette::GRAY_1),
                )
                .widget_info(|| WidgetInfo::labeled(Role::Label, true, &title));
                ui.add_space(SPACE_3);
                ui.label(
                    plain(texts.get(ComparisonText::EmptyHint), FS_XS, Weight::Regular)
                        .color(palette::GRAY_2_TEXT),
                );
            });
        });
}
