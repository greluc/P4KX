use egui::{Id, Rect, Ui};
use p4kx_extraction::Archive;

use super::extraction::number;
use crate::theme::desktop::{SEARCH_WIDTH, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::text::plain;
use crate::{ChangeExtractionText as T, ComparisonSide, Extraction, ExtractionScope, Model};

const ACTIONS: [(ComparisonSide, T); 2] = [
    (ComparisonSide::Old, T::ActionOld),
    (ComparisonSide::New, T::ActionNew),
];

/// Returns whether the actions, as wide as last drawn, still leave the search field its base
/// width in the rest of the row; otherwise the toolbar wraps them onto a row of their own.
pub(super) fn fit_beside_search(ui: &Ui) -> bool {
    let width = ui
        .data(|data| data.get_temp::<f32>(width_id()))
        .unwrap_or(0.0);
    ui.available_width() - width - TOOLBAR_PADDING >= SEARCH_WIDTH
}

/// Shows the actions that extract the changes of one side as ghosts in a right-to-left layout,
/// old left of new; returns the side whose action was clicked.
pub(super) fn actions(ui: &mut Ui, model: &Model) -> Option<ComparisonSide> {
    let texts = model.texts();
    let running = model.extraction().is_some_and(Extraction::is_running);
    let mut clicked = None;
    let mut drawn = Rect::NOTHING;
    for (side, text) in ACTIONS.into_iter().rev() {
        let len = model.scope_len(ExtractionScope::Changes(side));
        let label = texts.format(text, &[("number", number(texts, len))]);
        let response = button::button(ui, Kind::Ghost, Size::Normal, &label, !running && len > 0);
        drawn = drawn.union(response.rect);
        if response.clicked() {
            clicked = Some(side);
        }
    }
    let width = drawn.width() + ui.spacing().item_spacing.x;
    ui.data_mut(|data| data.insert_temp(width_id(), width));
    clicked
}

fn width_id() -> Id {
    Id::unique("comparison-extraction-actions")
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
    let comparison = model.comparison().view()?.comparison();
    Some(match side {
        ComparisonSide::Old => comparison.old_archive(),
        ComparisonSide::New => comparison.new_archive(),
    })
}
