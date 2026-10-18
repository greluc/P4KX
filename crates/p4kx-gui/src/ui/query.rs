use std::sync::Arc;

use egui::{Align, Frame, Label, Layout, Margin, Panel, Stroke, Ui, vec2};

use super::{Dialog, Requests, empty, query_editor, query_table};
use crate::theme::desktop::{
    COMPARISON_PROGRESS_WIDTH, LIST_FOOTER_PADDING_X, LIST_FOOTER_PADDING_Y, SPINNER_INLINE,
    TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::pagination::pagination;
use crate::widgets::text::hairline_below;
use crate::{Model, QueryExportFormat, QueryResult, QueryStatus, QueryTab, QueryText};

mod notes;

use notes::Notes;

/// Shows the Query tab: the editor with its actions and progress, the error of a Query that
/// cannot run, and the result with its export and pages.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(QueryText::EmptyTitle);
        let hint = texts.get(QueryText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    let notes = Notes::get(ui, model.texts(), model.query());
    let toolbar = frames::panel()
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| editor(ui, model, &notes));
    hairline_below(ui, toolbar.response.rect);
    frames::panel().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        results(ui, model, &notes, requests);
    });
}

fn editor(ui: &mut Ui, model: &mut Model, notes: &Notes) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let label = notes.show_label(ui);
    let mark = model.query().mark();

    let edited = query_editor::show(
        ui,
        model.query_mut().text_mut(),
        (&notes.hint, label.id),
        mark.as_ref(),
    );
    if let Some(alert) = &notes.alert {
        frames::danger_alert(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(Label::new(Arc::clone(alert)).selectable(true));
        });
    }
    let mut run = edited.submitted;
    let mut cancel = false;
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if model.query().is_running() {
                cancel =
                    button::button(ui, Kind::Ghost, Size::Normal, &notes.cancel, true).clicked();
            } else {
                let enabled = !model.query().text().trim().is_empty();
                run |= button::button(ui, Kind::Cta, Size::Normal, &notes.run, enabled).clicked();
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                progress(ui, model.query(), notes);
            });
        });
    });
    if cancel {
        model.query_mut().cancel();
    } else if run {
        let ctx = ui.ctx().clone();
        model.run_query(move || ctx.request_repaint());
    }
}

fn progress(ui: &mut Ui, query: &QueryTab, notes: &Notes) {
    if let QueryStatus::Running(_) = query.status() {
        match notes.fraction {
            Some(fraction) => {
                let width = ui.available_width().min(COMPARISON_PROGRESS_WIDTH);
                indicator::progress(ui, fraction, false, width);
            }
            None => indicator::spinner(ui, SPINNER_INLINE),
        }
    }
    if let Some(status) = &notes.status {
        ui.add(Label::new(Arc::clone(status)).selectable(false));
    }
}

fn results(ui: &mut Ui, model: &mut Model, notes: &Notes, requests: &mut Requests) {
    let Some(result) = model.query().result() else {
        prompt(ui, notes);
        return;
    };
    let exporting = model.query().is_exporting();
    let cancel_export = export_bar(ui, result, notes, (exporting, requests));
    let page = footer(ui, result, notes);
    let sort = if result.is_empty() {
        prompt(ui, notes);
        None
    } else {
        let laid = query_table::Laid::get(ui, model.texts(), result);
        query_table::show(ui, result, &laid)
    };
    if cancel_export {
        model.query_mut().cancel_export();
    }
    let Some(result) = model.query_mut().result_mut() else {
        return;
    };
    if let Some(column) = sort {
        result.sort_by(column);
    }
    if let Some(page) = page {
        result.set_page(page);
    }
}

fn prompt(ui: &mut Ui, notes: &Notes) {
    if let Some(prompt) = &notes.prompt {
        frames::preview_body().show(ui, |ui| {
            ui.add(Label::new(Arc::clone(prompt)));
        });
    }
}

/// Shows the export actions and the export note above the table; returns whether the running
/// export was asked to stop.
fn export_bar(
    ui: &mut Ui,
    result: &QueryResult,
    notes: &Notes,
    (exporting, requests): (bool, &mut Requests),
) -> bool {
    let mut cancel = false;
    let bar = Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
            ui.horizontal(|ui| {
                if let Some(export) = &notes.export {
                    ui.add(Label::new(Arc::clone(export)).selectable(true));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if exporting {
                        cancel = button::button(
                            ui,
                            Kind::Ghost,
                            Size::Normal,
                            &notes.export_cancel,
                            true,
                        )
                        .clicked();
                        return;
                    }
                    let enabled = !result.is_empty();
                    for (format, label) in [
                        (QueryExportFormat::Json, &notes.export_json),
                        (QueryExportFormat::Csv, &notes.export_csv),
                    ] {
                        if button::button(ui, Kind::Ghost, Size::Normal, label, enabled).clicked() {
                            requests.dialog = Some(Dialog::QueryExport(format));
                        }
                    }
                });
            });
        });
    hairline_below(ui, bar.response.rect);
    cancel
}

/// Shows the `.list-panel__footer` below the table: the number of Records and the pages;
/// returns the page asked for.
fn footer(ui: &mut Ui, result: &QueryResult, notes: &Notes) -> Option<usize> {
    let mut page = None;
    Panel::bottom("query-footer")
        .frame(Frame::NONE.inner_margin(Margin::symmetric(
            px(LIST_FOOTER_PADDING_X),
            px(LIST_FOOTER_PADDING_Y),
        )))
        .show_separator_line(false)
        .resizable(false)
        .show(ui, |ui| {
            let top = ui.max_rect().top() - LIST_FOOTER_PADDING_Y;
            ui.painter().hline(
                ui.max_rect()
                    .expand2(vec2(LIST_FOOTER_PADDING_X, 0.0))
                    .x_range(),
                top + 0.5,
                Stroke::new(1.0, palette::GRAY_3),
            );
            ui.horizontal(|ui| {
                if let Some(count) = &notes.count {
                    ui.add(Label::new(Arc::clone(count)).selectable(false));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if result.page_count() > 1
                        && let Some(status) = &notes.page_status
                    {
                        page = pagination(
                            ui,
                            (result.page(), result.page_count()),
                            Arc::clone(status),
                            notes.page_names(),
                        );
                    }
                });
            });
        });
    page
}
