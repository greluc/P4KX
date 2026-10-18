use egui::{Id, Ui, UiBuilder};
use p4kx_extraction::Archive;

use super::extraction::number;
use crate::comparison::side_archive;
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::text::plain;
use crate::{
    ChangeExtractionText as T, ComparisonSide, ComparisonView, Extraction, ExtractionScope, Model,
    Texts,
};

const ACTIONS: [(ComparisonSide, T); 2] = [
    (ComparisonSide::Old, T::ActionOld),
    (ComparisonSide::New, T::ActionNew),
];

/// Returns the labels of the actions, old side first, with the number of Entries each extracts.
pub(super) fn labels(texts: &Texts, view: &ComparisonView) -> [String; 2] {
    ACTIONS.map(|(side, text)| {
        let len = view.change_count(side);
        texts.format(text, &[("number", number(texts, len))])
    })
}

/// Shows the actions that extract the changes of one side as ghosts, old then new, each in a
/// scope of its own so that it keeps its id and focus wherever the toolbar places it; returns
/// the side whose action was clicked.
pub(super) fn actions(ui: &mut Ui, model: &Model, labels: &[String; 2]) -> Option<ComparisonSide> {
    let running = model.extraction().is_some_and(Extraction::is_running);
    let mut clicked = None;
    for ((side, _), label) in ACTIONS.into_iter().zip(labels) {
        let enabled = !running && model.scope_len(ExtractionScope::Changes(side)) > 0;
        let scope = UiBuilder::new().scope_id(Id::unique(("comparison-extraction", side)));
        let response = ui
            .scope_builder(scope, |ui| {
                button::button(ui, Kind::Ghost, Size::Normal, label, enabled)
            })
            .inner;
        if response.clicked() {
            clicked = Some(side);
        }
    }
    clicked
}

/// Returns the width both actions take with the gap between them.
pub(super) fn width(ui: &Ui, labels: &[String; 2]) -> f32 {
    let [old, new] = labels;
    button::width(ui, Size::Normal, old)
        + ui.spacing().item_spacing.x
        + button::width(ui, Size::Normal, new)
}

/// Shows the scope of the setup for the changes of `side` and the Filter that narrows it.
pub(super) fn scope(ui: &mut Ui, model: &Model, side: ComparisonSide) {
    let texts = model.texts();
    let text = match side {
        ComparisonSide::Old => T::ScopeOld,
        ComparisonSide::New => T::ScopeNew,
    };
    let len = model.scope_len(ExtractionScope::Changes(side));
    ui.label(texts.format(text, &[("number", number(texts, len))]));
    let filter = model
        .comparison()
        .view()
        .map_or("", |view| view.filter_text());
    if !filter.is_empty() {
        let note = texts.format(T::ScopeFiltered, &[("filter", filter.to_owned().into())]);
        ui.label(plain(note, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT));
    }
}

/// Returns the Archive of `side` of the finished Comparison.
pub(super) fn archive(model: &Model, side: ComparisonSide) -> Option<&Archive> {
    let view = model.comparison().view()?;
    Some(side_archive(view.comparison(), side))
}
