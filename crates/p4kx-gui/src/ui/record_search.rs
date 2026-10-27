// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Frame, Label, Layout, Margin, Sense, Ui, Vec2, vec2};
use fluent_bundle::FluentValue;

use super::dataforge_summary::message;
use super::extraction;
use super::record_results::{self, Laid, Note};
use super::search::field;
use crate::texts::RecordSearchText;
use crate::theme::desktop::{SEARCH_WIDTH, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::widgets::frames::{self, px};
use crate::widgets::indicator::spinner;
use crate::widgets::text::hairline_below;
use crate::{
    Browser, DataForgeText, Model, PreviewTab, RecordId, RecordTree, RecordTreeStatus, Text, Texts,
    format,
};

/// Shows the Record search in the list panel: the toolbar with the search field and the
/// Extraction actions, then the matching Records.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(laid) = model
        .browser()
        .and_then(|browser| Some((browser, browser.records()?)))
        .map(|(browser, tree)| Laid::get(ui, model.texts(), browser, tree))
    else {
        return;
    };
    let changed = toolbar(ui, model, &laid);
    let Some(browser) = model.browser() else {
        return;
    };
    let Some(tree) = browser.records() else {
        return;
    };
    let picked = body(ui, model.texts(), browser, tree, &laid);
    let ctx = ui.ctx().clone();
    if picked.is_some() {
        model.set_preview_tab(PreviewTab::Record);
    }
    let Some(browser) = model.browser_mut() else {
        return;
    };
    if let Some(text) = changed {
        browser.search_records(&text, move || ctx.request_repaint());
    }
    if let Some(id) = picked {
        browser.pick_record(id);
    }
}

fn toolbar(ui: &mut Ui, model: &mut Model, laid: &Laid) -> Option<String> {
    let mut changed = None;
    let toolbar = Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
            let mut stacked = false;
            ui.horizontal(|ui| {
                let label = laid.show_label(ui);
                let hint = laid.hint();
                stacked = !extraction::fit_beside(ui, model, SEARCH_WIDTH);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !stacked {
                        extraction::actions(ui, model);
                    }
                    let Some(browser) = model.browser() else {
                        return;
                    };
                    progress(ui, model.texts(), browser, laid);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        laid.edit(browser.record_search_text(), |text| {
                            if field(ui, text, &hint, label.id) {
                                changed = Some(text.clone());
                            }
                        });
                    });
                });
            });
            if stacked {
                extraction::stacked_actions(ui, model);
            }
        });
    hairline_below(ui, toolbar.response.rect);
    changed
}
fn progress(ui: &mut Ui, texts: &Texts, browser: &Browser, laid: &Laid) {
    let searching = browser.is_searching_records();
    let matches = browser.record_matches();
    let note = if searching {
        Note::Searching
    } else {
        Note::Count
    };
    let galley = laid.note(ui, note, || match (searching, matches) {
        (true, _) => (texts.get(Text::StatusSearching), palette::PRIMARY),
        (false, Some(matches)) => {
            let count = u64::try_from(matches.len()).unwrap_or(u64::MAX);
            let text = texts.format(
                RecordSearchText::Matches,
                &[
                    ("count", FluentValue::from(count)),
                    ("number", format::number(count, texts.language()).into()),
                ],
            );
            (text, palette::GRAY_2_TEXT)
        }
        (false, None) => (String::new(), palette::GRAY_2_TEXT),
    });
    ui.add(Label::new(galley).selectable(false));
    if searching {
        spinner(ui, SPINNER_INLINE);
    } else {
        ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
    }
}

fn body(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    tree: &RecordTree,
    laid: &Laid,
) -> Option<RecordId> {
    let searched = !browser.record_search_text().is_empty();
    let loading = matches!(tree.status(), RecordTreeStatus::Loading) && searched;
    let galley = match browser.record_matches() {
        Some(matches) if !matches.is_empty() => {
            return record_results::show(ui, tree, laid, matches);
        }
        Some(_) => laid.note(ui, Note::NoMatches, || {
            (texts.get(RecordSearchText::NoMatches), palette::GRAY_2_TEXT)
        }),
        None if loading => laid.note(ui, Note::Status, || {
            let path = ("path", tree.entry_path().to_owned().into());
            (
                texts.format(DataForgeText::Loading, &[path]),
                palette::GRAY_1,
            )
        }),
        None => laid.note(ui, Note::Prompt, || {
            message(texts, tree.entry_path(), tree.status())
                .unwrap_or_else(|| (texts.get(RecordSearchText::Prompt), palette::GRAY_2_TEXT))
        }),
    };
    frames::preview_body().show(ui, |ui| {
        ui.horizontal(|ui| {
            if loading {
                spinner(ui, SPINNER_INLINE);
            }
            ui.add(Label::new(galley));
        });
    });
    ui.allocate_space(ui.available_size());
    None
}
