use std::sync::Arc;

use egui::{Align, Frame, Galley, Id, Layout, Margin, Ui, vec2};

use super::comparison_list::{self, ListAction};
use super::search::{eyebrow, field};
use super::{Dialog, Requests};
use crate::theme::desktop::TOOLBAR_PADDING;
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::px;
use crate::widgets::text::{hairline_below, line, plain};
use crate::{ComparisonText, ComparisonView, Model, ReportStatus, Text, Texts, format};

/// Shows the list panel of the changes: the Filter, the report and the sortable list.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    toolbar(ui, model, requests);
    let reveal = model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::take_reveal);
    let texts = model.texts();
    let Some(view) = model.comparison().view() else {
        return;
    };
    let action = comparison_list::show(ui, texts, view, reveal);
    let Some(view) = model.comparison_mut().view_mut() else {
        return;
    };
    match action {
        Some(ListAction::Sort(column)) => view.sort_by(column),
        Some(ListAction::Select(id)) => view.select(id),
        None => {}
    }
}

/// The texts of the toolbar, laid out again only when the language, the Comparison, the number
/// of shown changes or the report status change, and the edit buffer of the Filter.
#[derive(Clone, Default)]
struct ToolbarCache {
    key: Option<Id>,
    label: Option<Arc<Galley>>,
    hint: String,
    save: String,
    shown: Option<Arc<Galley>>,
    note: Option<Arc<Galley>>,
    filter: String,
}

impl ToolbarCache {
    fn load(ui: &Ui, texts: &Texts, view: &ComparisonView) -> Self {
        let id = Id::unique("comparison-toolbar");
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(id)));
        let key = Id::unique((
            texts.language(),
            view.generation(),
            view.shown().len(),
            view.report_status(),
        ));
        if cache.key != Some(key) {
            cache.key = Some(key);
            cache.label = Some(line(
                ui,
                eyebrow(&texts.get(Text::SearchLabel)),
                f32::INFINITY,
            ));
            cache.hint = texts.get(Text::SearchHint);
            cache.save = texts.get(ComparisonText::SaveReport);
            let shown = plain(shown_note(texts, view), FS_XS, Weight::Regular);
            cache.shown = Some(line(ui, shown.color(palette::GRAY_2_TEXT), f32::INFINITY));
            cache.note = report_note(texts, view).map(|(note, colour)| {
                let note = plain(note, FS_XS, Weight::Regular).color(colour);
                line(ui, note, f32::INFINITY)
            });
        }
        if cache.filter != view.filter_text() {
            cache.filter.clear();
            cache.filter.push_str(view.filter_text());
        }
        cache
    }

    fn store(self, ui: &Ui) {
        let id = Id::unique("comparison-toolbar");
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(id) = self);
    }
}

fn toolbar(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let texts = model.texts();
    let Some(view) = model.comparison().view() else {
        return;
    };
    let mut cache = ToolbarCache::load(ui, texts, view);
    let saving = view.is_saving();
    let mut changed = false;
    let frame = Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
            ui.horizontal(|ui| {
                let label = cache.label.clone().map(|label| ui.label(label));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button::button(ui, Kind::Ghost, Size::Normal, &cache.save, !saving).clicked()
                    {
                        requests.dialog = Some(Dialog::ComparisonReport);
                    }
                    if let Some(shown) = cache.shown.clone() {
                        ui.label(shown);
                    }
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let label = label.map_or_else(|| ui.next_auto_id(), |label| label.id);
                        changed |= field(ui, &mut cache.filter, &cache.hint, label);
                    });
                });
            });
            if let Some(note) = cache.note.clone() {
                ui.label(note);
            }
        });
    hairline_below(ui, frame.response.rect);
    if changed {
        let ctx = ui.ctx().clone();
        if let Some(view) = model.comparison_mut().view_mut() {
            view.set_filter(&cache.filter, move || ctx.request_repaint());
        }
    }
    cache.store(ui);
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
