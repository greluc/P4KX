use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Frame, Galley, Label, Layout, Margin, Response, Role, Sense, TextStyle, TextWrapMode,
    Ui, WidgetInfo, WidgetText, vec2,
};

use super::{Dialog, Requests, empty, query_editor, query_table};
use crate::theme::desktop::{COMPARISON_PROGRESS_WIDTH, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::pagination::pagination;
use crate::widgets::text::{hairline_below, line, plain};
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
    let mark = match model.query().status() {
        QueryStatus::Invalid(invalid) if invalid.query() == model.query().text() => {
            Some(invalid.span())
        }
        _ => None,
    };
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
    let page = results_bar(ui, result, notes, (exporting, requests));
    let sort = if result.is_empty() {
        prompt(ui, notes);
        None
    } else {
        let laid = query_table::Laid::get(ui, model.texts(), result);
        query_table::show(ui, result, &laid)
    };
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

/// Shows the count, the pages and the export actions above the table; returns the page asked
/// for.
fn results_bar(
    ui: &mut Ui,
    result: &QueryResult,
    notes: &Notes,
    (exporting, requests): (bool, &mut Requests),
) -> Option<usize> {
    let mut page = None;
    let bar = Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
            ui.horizontal(|ui| {
                if let Some(count) = &notes.count {
                    ui.add(Label::new(Arc::clone(count)).selectable(false));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let enabled = !exporting && !result.is_empty();
                    for (format, label) in [
                        (QueryExportFormat::Json, &notes.export_json),
                        (QueryExportFormat::Csv, &notes.export_csv),
                    ] {
                        if button::button(ui, Kind::Ghost, Size::Normal, label, enabled).clicked() {
                            requests.dialog = Some(Dialog::QueryExport(format));
                        }
                    }
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
            if let Some(export) = &notes.export {
                ui.add(Label::new(Arc::clone(export)).selectable(true));
            }
        });
    hairline_below(ui, bar.response.rect);
    page
}
/// Returns `status` laid out as the `.page-status`: the current page in bold orange, then the
/// number of pages.
fn page_status(ui: &Ui, page: usize, pages: usize) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    job.append(
        &(page + 1).to_string(),
        0.0,
        TextFormat::simple(font(FS_XS, Weight::Bold), palette::PRIMARY),
    );
    job.append(
        &format!(" / {pages}"),
        0.0,
        TextFormat::simple(font(FS_XS, Weight::Regular), palette::GRAY_1),
    );
    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
}

/// Lays `text` out in `colour`, wrapped at `width`.
fn wrapped(ui: &Ui, text: String, size: f32, colour: egui::Color32, width: f32) -> Arc<Galley> {
    WidgetText::from(plain(text, size, Weight::Regular).color(colour)).into_galley(
        ui,
        Some(TextWrapMode::Wrap),
        width,
        TextStyle::Body,
    )
}

/// Lays `text` out on one line in `colour` at the note size.
fn note(ui: &Ui, text: String, colour: egui::Color32) -> Arc<Galley> {
    line(
        ui,
        plain(text, FS_XS, Weight::Regular).color(colour),
        f32::INFINITY,
    )
}

/// Shows the label of the editor as an eyebrow named by its Fluent text.
fn labelled(ui: &mut Ui, (name, galley): &(Arc<str>, Arc<Galley>)) -> Response {
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &**name));
    ui.painter()
        .galley(rect.min, Arc::clone(galley), palette::GRAY_2_TEXT);
    response
}

/// Hashes what decides the texts of the tab.
fn status_key(query: &QueryTab, hasher: &mut impl Hasher) {
    match query.status() {
        QueryStatus::Idle => 0_u8.hash(hasher),
        QueryStatus::Running(stage) => {
            1_u8.hash(hasher);
            stage.hash(hasher);
        }
        QueryStatus::Invalid(invalid) => {
            2_u8.hash(hasher);
            invalid.hash(hasher);
        }
        QueryStatus::Cancelled => 3_u8.hash(hasher),
        QueryStatus::Failed(reason) => {
            4_u8.hash(hasher);
            reason.hash(hasher);
        }
        QueryStatus::ArchiveChanged => 5_u8.hash(hasher),
        QueryStatus::Finished(result) => {
            6_u8.hash(hasher);
            result.generation().hash(hasher);
        }
    }
    query.export_status().hash(hasher);
}
