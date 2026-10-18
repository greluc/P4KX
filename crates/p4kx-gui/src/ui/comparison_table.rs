use egui::{Align, Frame, Layout, Margin, Ui, vec2};

use super::comparison_list::{self, ListAction};
use super::search::{eyebrow, field};
use super::{Dialog, Requests};
use crate::theme::desktop::TOOLBAR_PADDING;
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::px;
use crate::widgets::text::{hairline_below, plain};
use crate::{ComparisonText, ComparisonView, Model, ReportStatus, Text, Texts, format};

/// Shows the list panel of the changes: the Filter, the report and the sortable list.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    toolbar(ui, model, requests);
    let texts = model.texts();
    let Some(view) = model.comparison().view() else {
        return;
    };
    let action = comparison_list::show(ui, texts, view);
    let Some(view) = model.comparison_mut().view_mut() else {
        return;
    };
    match action {
        Some(ListAction::Sort(column)) => view.sort_by(column),
        Some(ListAction::Select(id)) => view.select(id),
        None => {}
    }
}

fn toolbar(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let texts = model.texts();
    let Some(view) = model.comparison().view() else {
        return;
    };
    let mut text = view.filter_text().to_owned();
    let label = eyebrow(&texts.get(Text::SearchLabel));
    let hint = texts.get(Text::SearchHint);
    let save = texts.get(ComparisonText::SaveReport);
    let saving = view.is_saving();
    let shown = shown_note(texts, view);
    let note = report_note(texts, view);
    let mut changed = false;
    let frame = Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
            ui.horizontal(|ui| {
                let label = ui.label(label);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button::button(ui, Kind::Ghost, Size::Normal, &save, !saving).clicked() {
                        requests.dialog = Some(Dialog::ComparisonReport);
                    }
                    ui.label(plain(shown, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT));
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        changed |= field(ui, &mut text, &hint, label.id);
                    });
                });
            });
            if let Some((note, colour)) = note {
                ui.label(plain(note, FS_XS, Weight::Regular).color(colour));
            }
        });
    hairline_below(ui, frame.response.rect);
    if changed {
        let ctx = ui.ctx().clone();
        if let Some(view) = model.comparison_mut().view_mut() {
            view.set_filter(&text, move || ctx.request_repaint());
        }
    }
}

fn shown_note(texts: &Texts, view: &ComparisonView) -> String {
    let number = format::number(
        u64::try_from(view.shown().len()).unwrap_or(u64::MAX),
        texts.language(),
    );
    texts.format(ComparisonText::Shown, &[("number", number.into())])
}

fn report_note(texts: &Texts, view: &ComparisonView) -> Option<(String, egui::Color32)> {
    let path = |path: &std::path::Path| ("path", path.display().to_string().into());
    Some(match view.report_status()? {
        ReportStatus::Saving(at) => (
            texts.format(ComparisonText::ReportSaving, &[path(at)]),
            palette::PRIMARY,
        ),
        ReportStatus::Saved(at) => (
            texts.format(ComparisonText::ReportSaved, &[path(at)]),
            palette::GRAY_2_TEXT,
        ),
        ReportStatus::Failed(at, reason) => (
            texts.format(
                ComparisonText::ReportFailed,
                &[path(at), ("reason", reason.clone().into())],
            ),
            palette::DANGER_TEXT,
        ),
    })
}
