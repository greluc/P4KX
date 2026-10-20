use egui::{Align, Key, Label, Layout, Margin, Ui, vec2};
use fluent_bundle::FluentValue;
use p4kx_search::Source;

use super::content_search_rows;
use super::search::{eyebrow, field_response};
use super::{empty, list_footer};
use crate::theme::desktop::{SPINNER, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::text::{hairline_below, plain};
use crate::widgets::{indicator, tab, toggle};
use crate::{ContentResult, ContentSearchStatus, ContentSearchText, Model, Texts, format};

const SOURCES: [(Source, ContentSearchText); 3] = [
    (Source::Records, ContentSearchText::SourceRecords),
    (Source::Localization, ContentSearchText::SourceLocalization),
    (Source::Entries, ContentSearchText::SourceEntries),
];

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
    let toolbar = frames::panel()
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| toolbar(ui, model));
    hairline_below(ui, toolbar.response.rect);
    let action = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            body(ui, model)
        })
        .inner;
    match action {
        Some(Action::Open(position)) => {
            let ctx = ui.ctx().clone();
            model.open_hit(position, move || ctx.request_repaint());
        }
        Some(Action::Show(source)) => model.content_search_mut().show(source),
        None => {}
    }
}

/// What the user did in the results.
pub(super) enum Action {
    Open(usize),
    Show(Source),
}

fn toolbar(ui: &mut Ui, model: &mut Model) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let texts = model.texts();
    let label = eyebrow(&texts.get(ContentSearchText::Label));
    let hint = texts.get(ContentSearchText::Hint);
    let run = texts.get(ContentSearchText::Run);
    let cancel = texts.get(ContentSearchText::Cancel);
    let sources_label = eyebrow(&texts.get(ContentSearchText::SourcesLabel));
    let names = SOURCES.map(|(_, text)| texts.get(text));
    let running = model.content_search().is_running();
    let mut text = model.content_search().text().to_owned();
    let mut start = false;
    let mut stop = false;
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if running {
                stop = button::button(ui, Kind::Ghost, Size::Normal, &cancel, true).clicked();
            } else {
                let enabled = !text.is_empty() && !model.content_search().sources().is_empty();
                start = button::button(ui, Kind::Cta, Size::Normal, &run, enabled).clicked();
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let response = field_response(ui, &mut text, &hint, label.id);
                start |= !running
                    && response.lost_focus()
                    && ui.input(|input| input.key_pressed(Key::Enter));
            });
        });
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(sources_label);
        for ((source, _), name) in SOURCES.into_iter().zip(&names) {
            let mut on = model.content_search().sources().contains(source);
            if toggle::checkbox(ui, &mut on, name).changed() {
                model.content_search_mut().set_source(source, on);
            }
        }
    });
    model.content_search_mut().set_text(&text);
    if stop {
        model.content_search_mut().cancel();
    }
    if start {
        let ctx = ui.ctx().clone();
        model.run_content_search(move || ctx.request_repaint());
    }
}

fn body(ui: &mut Ui, model: &Model) -> Option<Action> {
    let texts = model.texts();
    let tab = model.content_search();
    match tab.status() {
        ContentSearchStatus::Idle => message(
            ui,
            texts.get(ContentSearchText::Prompt),
            palette::GRAY_2_TEXT,
        ),
        ContentSearchStatus::Running => searching(ui, texts),
        ContentSearchStatus::Cancelled => message(
            ui,
            texts.get(ContentSearchText::Cancelled),
            palette::GRAY_2_TEXT,
        ),
        ContentSearchStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.get(ContentSearchText::Stopped),
                |reason| {
                    texts.format(
                        ContentSearchText::Failed,
                        &[("reason", reason.to_owned().into())],
                    )
                },
            );
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        ContentSearchStatus::ArchiveChanged => {
            let text = texts.get(ContentSearchText::ArchiveChanged);
            frames::warning_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        ContentSearchStatus::Finished(result) => return finished(ui, model, result),
    }
    None
}

fn finished(ui: &mut Ui, model: &Model, result: &ContentResult) -> Option<Action> {
    let texts = model.texts();
    let tab = model.content_search();
    list_footer::show(ui, "content-search-footer", |ui| {
        let seconds = format!("{:.2}", result.elapsed().as_secs_f64());
        let hits = u64::try_from(result.len(Source::Records) + result.len(Source::Localization))
            .unwrap_or(u64::MAX)
            .saturating_add(u64::try_from(result.len(Source::Entries)).unwrap_or(u64::MAX));
        let text = texts.format(
            ContentSearchText::Hits,
            &[
                ("count", FluentValue::from(hits)),
                ("number", format::number(hits, texts.language()).into()),
                ("seconds", seconds.into()),
            ],
        );
        ui.add(
            Label::new(plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    });
    notes(ui, texts, result);
    if result.is_empty() {
        let text = texts.format(
            ContentSearchText::NoMatch,
            &[("text", result.text().to_owned().into())],
        );
        message(ui, text, palette::GRAY_2_TEXT);
        return None;
    }
    let labels = SOURCES.map(|(source, text)| segment(texts, result, source, text));
    let active = SOURCES
        .iter()
        .position(|(source, _)| *source == tab.shown());
    let mut shown = None;
    ui.horizontal(|ui| {
        ui.add_space(TOOLBAR_PADDING);
        shown = tab::segments(ui, &labels, active)
            .and_then(|index| SOURCES.get(index))
            .map(|(source, _)| Action::Show(*source));
    });
    ui.add_space(TOOLBAR_PADDING);
    let opened = content_search_rows::show(ui, texts, result, tab).map(Action::Open);
    shown.or(opened)
}

fn segment(
    texts: &Texts,
    result: &ContentResult,
    source: Source,
    name: ContentSearchText,
) -> String {
    let count = u64::try_from(result.len(source)).unwrap_or(u64::MAX);
    let text = if result.has_more(source) {
        ContentSearchText::SegmentMore
    } else {
        ContentSearchText::Segment
    };
    texts.format(
        text,
        &[
            ("name", texts.get(name).into()),
            ("number", format::number(count, texts.language()).into()),
        ],
    )
}

fn notes(ui: &mut Ui, texts: &Texts, result: &ContentResult) {
    let stats = result.stats();
    let mut lines = Vec::new();
    let more = u64::try_from(p4kx_search::DEFAULT_LIMIT).unwrap_or(u64::MAX);
    if Source::ALL
        .into_iter()
        .any(|source| result.has_more(source))
    {
        lines.push(texts.format(
            ContentSearchText::More,
            &[("number", format::number(more, texts.language()).into())],
        ));
    }
    if result
        .unavailable()
        .iter()
        .any(|missing| missing.source == Source::Records)
    {
        lines.push(texts.get(ContentSearchText::NoDataForge));
    }
    let skipped = stats.entries_skipped();
    if skipped > 0 {
        let number = |value: u64| format::number(value, texts.language());
        lines.push(texts.format(
            ContentSearchText::Skipped,
            &[
                ("count", FluentValue::from(skipped)),
                ("number", number(skipped).into()),
                ("binary", number(stats.entries_binary).into()),
                ("large", number(stats.entries_too_large).into()),
                ("key", number(stats.entries_key_required).into()),
                ("unreadable", number(stats.entries_unreadable).into()),
            ],
        ));
    }
    for line in lines {
        frames::warning_alert(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(Label::new(line).selectable(true));
        });
    }
}

fn searching(ui: &mut Ui, texts: &Texts) {
    ui.add_space(SPINNER);
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.add(
            Label::new(
                plain(
                    texts.get(ContentSearchText::Searching),
                    FS_SM,
                    Weight::Regular,
                )
                .color(palette::GRAY_1),
            )
            .selectable(false),
        );
    });
}

fn message(ui: &mut Ui, text: String, colour: egui::Color32) {
    frames::preview_body().show(ui, |ui| {
        ui.add(Label::new(
            plain(text, FS_SM, Weight::Regular).color(colour),
        ));
    });
}
