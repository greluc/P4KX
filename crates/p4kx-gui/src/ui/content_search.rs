use std::sync::Arc;

use egui::{Align, Frame, Galley, Key, Label, Layout, Margin, Ui, vec2};
use p4kx_search::Source;

use super::content_search_notes::{Body, Finished, Notes, SOURCES, labelled};
use super::content_search_rows;
use super::search::field_response;
use super::{empty, list_footer};
use crate::theme::desktop::{SPINNER, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::tokens::SPACE_6;
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::text::hairline_below;
use crate::widgets::{indicator, tab, toggle};
use crate::{ContentSearchText, KeyReason, Model};

/// Shows the Search tab: the search field and the sources, then the hits of one source with the
/// notes about what was left out, or the state of the search.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(ContentSearchText::EmptyTitle);
        let hint = texts.get(ContentSearchText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if model.content_search().is_running() {
        ui.ctx().request_repaint();
    }
    let notes = Notes::get(
        ui,
        model.texts(),
        model.content_search(),
        ui.available_width(),
    );
    let toolbar = frames::panel()
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| toolbar(ui, model, &notes));
    hairline_below(ui, toolbar.response.rect);
    let action = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            body(ui, model, &notes)
        })
        .inner;
    match action {
        Some(Action::Open(position)) => {
            let ctx = ui.ctx().clone();
            model.open_hit(position, move || ctx.request_repaint());
        }
        Some(Action::Show(source)) => model.content_search_mut().show(source),
        Some(Action::EnterKey) => model.open_key_dialog(KeyReason::Menu),
        None => {}
    }
}

/// What the user did in the results.
pub(super) enum Action {
    Open(usize),
    Show(Source),
    EnterKey,
}

fn toolbar(ui: &mut Ui, model: &mut Model, notes: &Notes) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let running = model.content_search().is_running();
    let enabled =
        !model.content_search().text().is_empty() && !model.content_search().sources().is_empty();
    let mut start = false;
    let mut stop = false;
    ui.horizontal(|ui| {
        let label = labelled(ui, &notes.label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if running {
                let (name, galley) = &notes.cancel;
                let laid = (&**name, Arc::clone(galley));
                stop = button::button_laid(ui, (Kind::Ghost, Size::Normal), laid, true).clicked();
            } else {
                let (name, galley) = &notes.run;
                let laid = (&**name, Arc::clone(galley));
                start = button::button_laid(ui, (Kind::Cta, Size::Normal), laid, enabled).clicked();
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let text = model.content_search_mut().text_mut();
                let response = field_response(ui, text, &notes.hint, label.id);
                start |= !running
                    && response.lost_focus()
                    && ui.input(|input| input.key_pressed(Key::Enter));
            });
        });
    });
    ui.horizontal_wrapped(|ui| {
        show_galley(ui, &notes.sources_label, false);
        for ((source, _), name) in SOURCES.into_iter().zip(&notes.names) {
            let mut on = model.content_search().sources().contains(source);
            if toggle::checkbox(ui, &mut on, name).changed() {
                model.content_search_mut().set_source(source, on);
            }
        }
    });
    if stop {
        model.content_search_mut().cancel();
    }
    if start {
        let ctx = ui.ctx().clone();
        model.run_content_search(move || ctx.request_repaint());
    }
}

fn show_galley(ui: &mut Ui, galley: &Arc<Galley>, selectable: bool) {
    ui.add(Label::new(Arc::clone(galley)).selectable(selectable));
}

fn body(ui: &mut Ui, model: &Model, notes: &Notes) -> Option<Action> {
    match &notes.body {
        Body::Message(galley) => message(ui, galley),
        Body::Searching(galley) => searching(ui, galley),
        Body::Danger(galley) => inset_alert(ui, galley, true),
        Body::Warning(galley) => inset_alert(ui, galley, false),
        Body::Finished(finished) => return results(ui, model, finished),
    }
    None
}

fn results(ui: &mut Ui, model: &Model, finished: &Finished) -> Option<Action> {
    let tab = model.content_search();
    list_footer::show(ui, "content-search-footer", |ui| {
        show_galley(ui, &finished.hits, false);
    });
    let enter_key = alert(ui, finished);
    let Some(labels) = &finished.segments else {
        if let Some(galley) = &finished.message {
            message(ui, galley);
        }
        return enter_key;
    };
    let active = SOURCES
        .iter()
        .position(|(source, _)| *source == tab.shown());
    let mut shown = None;
    ui.horizontal(|ui| {
        ui.add_space(TOOLBAR_PADDING);
        shown = tab::counted_segments(ui, labels, active)
            .and_then(|index| SOURCES.get(index))
            .map(|(source, _)| Action::Show(*source));
    });
    ui.add_space(TOOLBAR_PADDING);
    if let Some(galley) = &finished.message {
        message(ui, galley);
        return enter_key.or(shown);
    }
    let result = tab.result()?;
    let opened = content_search_rows::show(ui, model.texts(), result, tab).map(Action::Open);
    enter_key.or(shown).or(opened)
}

fn alert(ui: &mut Ui, finished: &Finished) -> Option<Action> {
    if finished.notes.is_empty() && finished.key.is_none() {
        return None;
    }
    let mut action = None;
    inset().show(ui, |ui| {
        frames::warning_alert(ui, |ui| {
            for galley in &finished.notes {
                show_galley(ui, galley, true);
            }
            if let Some((galley, (name, label))) = &finished.key {
                show_galley(ui, galley, true);
                let laid = (&**name, Arc::clone(label));
                let button = button::button_laid(ui, (Kind::Ghost, Size::Compact), laid, true);
                if button.clicked() {
                    action = Some(Action::EnterKey);
                }
            }
        });
    });
    action
}

fn inset() -> Frame {
    Frame::NONE.inner_margin(Margin {
        left: px(TOOLBAR_PADDING),
        right: px(TOOLBAR_PADDING),
        top: px(TOOLBAR_PADDING),
        bottom: px(SPACE_6),
    })
}

fn inset_alert(ui: &mut Ui, galley: &Arc<Galley>, danger: bool) {
    inset().show(ui, |ui| {
        let text = |ui: &mut Ui| show_galley(ui, galley, true);
        if danger {
            frames::danger_alert(ui, text);
        } else {
            frames::warning_alert(ui, text);
        }
    });
}

fn searching(ui: &mut Ui, galley: &Arc<Galley>) {
    frames::preview_body().show(ui, |ui| {
        ui.add_space(SPINNER);
        ui.horizontal(|ui| {
            indicator::spinner(ui, SPINNER_INLINE);
            show_galley(ui, galley, false);
        });
    });
}

fn message(ui: &mut Ui, galley: &Arc<Galley>) {
    frames::preview_body().show(ui, |ui| {
        show_galley(ui, galley, false);
    });
}
