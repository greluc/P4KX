use std::sync::Arc;

use egui::{Id, Ui, UiBuilder};

use super::record_list::{self, RecordAction};
use crate::widgets::tab::{self, TabLabel};
use crate::{
    ComparisonList, ComparisonText, ComparisonView, Model, RecordChangeView, SchemaText, Text,
    Texts, format,
};

const LISTS: [ComparisonList; 3] = [
    ComparisonList::Entries,
    ComparisonList::Records,
    ComparisonList::Schema,
];

/// Shows the `.tab-nav` that switches between the Entry and the Record list once the Record
/// level was compared.
pub(super) fn tabs(ui: &mut Ui, model: &mut Model) {
    let Some(view) = model.comparison().view() else {
        return;
    };
    if view.records().is_none() {
        return;
    }
    let labels = labels(ui, model.texts(), view);
    let current = LISTS
        .iter()
        .position(|&list| list == view.list())
        .unwrap_or(0);
    let clicked = ui
        .scope_builder(UiBuilder::new().id_salt("comparison-lists"), |ui| {
            tab::tabs(ui, &labels, current)
        })
        .inner
        .and_then(|index| LISTS.get(index).copied());
    if let (Some(list), Some(view)) = (clicked, model.comparison_mut().view_mut()) {
        view.show_list(list);
    }
}

/// Returns both tabs with the number of shown Entries and Records, laid out again only when the
/// language, the Comparison or a number changes.
fn labels(ui: &Ui, texts: &Texts, view: &ComparisonView) -> Arc<[TabLabel]> {
    let records = view.records().map_or(0, |records| records.shown().len());
    let schema = view.schema().map(|schema| schema.rows().len());
    let key = Id::unique((
        texts.language(),
        view.generation(),
        view.shown().len(),
        records,
        schema,
        ui.ctx().pixels_per_point().to_bits(),
    ));
    let id = Id::unique("comparison-list-tabs");
    if let Some((known, labels)) = ui.data(|data| data.get_temp::<(Id, Arc<[TabLabel]>)>(id))
        && known == key
    {
        return labels;
    }
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let tab = |text: Text, count: usize| TabLabel::new(ui, &texts.get(text), Some(&number(count)));
    let mut labels = vec![
        tab(ComparisonText::LevelEntries.into(), view.shown().len()),
        tab(ComparisonText::LevelRecords.into(), records),
    ];
    labels.extend(schema.map(|count| tab(SchemaText::Tab.into(), count)));
    let labels: Arc<[TabLabel]> = labels.into();
    ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&labels))));
    labels
}

/// Returns whether the Record list is shown.
pub(super) fn records_shown(view: &ComparisonView) -> bool {
    view.list() == ComparisonList::Records && view.records().is_some()
}

/// Returns whether the schema list is shown.
pub(super) fn schema_shown(view: &ComparisonView) -> bool {
    view.list() == ComparisonList::Schema && view.schema().is_some()
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
