use std::sync::Arc;

use egui::{Id, Ui, UiBuilder};

use super::record_list::{self, RecordAction};
use crate::widgets::tab;
use crate::{ComparisonList, ComparisonText, ComparisonView, Language, Model, RecordChangeView};

const LISTS: [ComparisonList; 2] = [ComparisonList::Entries, ComparisonList::Records];

/// Shows the `.tab-nav` that switches between the Entry and the Record list once the Record
/// level was compared.
pub(super) fn tabs(ui: &mut Ui, model: &mut Model) {
    let Some(view) = model.comparison().view() else {
        return;
    };
    if view.records().is_none() {
        return;
    }
    let texts = model.texts();
    let id = Id::unique("comparison-list-tabs");
    let key = texts.language();
    let labels = match ui.data(|data| data.get_temp::<(Language, Arc<[String; 2]>)>(id)) {
        Some((known, labels)) if known == key => labels,
        _ => {
            let labels = Arc::new(
                [ComparisonText::LevelEntries, ComparisonText::LevelRecords]
                    .map(|text| texts.get(text)),
            );
            ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&labels))));
            labels
        }
    };
    let current = LISTS
        .iter()
        .position(|&list| list == view.list())
        .unwrap_or(0);
    let clicked = ui
        .scope_builder(UiBuilder::new().id_salt("comparison-lists"), |ui| {
            tab::tab_bar(ui, labels.as_slice(), current)
        })
        .inner
        .and_then(|index| LISTS.get(index).copied());
    if let (Some(list), Some(view)) = (clicked, model.comparison_mut().view_mut()) {
        view.show_list(list);
    }
}

/// Returns whether the Record list is shown.
pub(super) fn records_shown(view: &ComparisonView) -> bool {
    view.list() == ComparisonList::Records && view.records().is_some()
}

/// Shows the Record list and applies what the user did in it.
pub(super) fn records(ui: &mut Ui, model: &mut Model) {
    let reveal = model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::records_mut)
        .and_then(RecordChangeView::take_reveal);
    let texts = model.texts();
    let Some(view) = model.comparison().view() else {
        return;
    };
    let Some(records) = view.records() else {
        return;
    };
    let filtered = !view.filter_text().is_empty();
    let action = record_list::show(ui, texts, (records, view.generation()), (filtered, reveal));
    let Some(records) = model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::records_mut)
    else {
        return;
    };
    match action {
        Some(RecordAction::Sort(column)) => records.sort_by(column),
        Some(RecordAction::Select(id)) => records.select(id),
        None => {}
    }
}
