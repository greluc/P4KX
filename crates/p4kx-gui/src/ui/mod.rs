//! The egui code that draws a [`Model`] and turns clicks into changes of it.

mod archive_change;
mod archive_picker;
mod audio_controls;
mod audio_preview;
mod channels;
mod comparison;
mod comparison_diff;
mod comparison_extraction;
mod comparison_list;
mod comparison_lists;
mod comparison_rows;
mod comparison_summary;
mod comparison_table;
mod dataforge_summary;
mod diff_line;
mod diff_rows;
mod empty;
mod extraction;
mod extraction_outcome;
mod field_rows;
mod hex_view;
mod image_preview;
mod licence;
mod menu;
mod moved_path;
mod preview;
mod query;
mod query_editor;
mod query_table;
mod record_export;
mod record_export_outcome;
mod record_fields;
mod record_list;
mod record_results;
mod record_rows;
mod record_search;
mod record_tree;
mod search;
mod shortcuts;
mod status;
mod table;
mod tree;
mod xml_preview;
mod xml_view;

use std::path::PathBuf;

use egui::{CentralPanel, Margin, Panel, Ui};

use crate::theme::desktop::{
    HEADER_HEIGHT, PREVIEW_HEIGHT, PREVIEW_MIN_HEIGHT, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
    SIDEBAR_WIDTH, STATUS_HEIGHT,
};
use crate::widgets::{frames, tab};

const RESIZE_GAP: i8 = 6;
use crate::{ComparisonSide, ComparisonText, Model, QueryExportFormat, QueryText, Text, WorkTab};

/// A dialog the host shows for the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// Picks an Archive to open.
    OpenArchive,
    /// Picks a folder to add as a manual entry.
    AddFolder,
    /// Picks an Archive to add as a manual entry.
    AddArchive,
    /// Picks the folder of an Extraction.
    ExtractionFolder,
    /// Picks the Archive of one side of the Comparison.
    ComparisonArchive(ComparisonSide),
    /// Picks the file the Comparison report is saved to.
    ComparisonReport,
    /// Picks the file the Query result is exported to.
    QueryExport(QueryExportFormat),
    /// Picks the folder of a Record export.
    RecordExportFolder,
}

/// What the window asks of its host after a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requests {
    /// The dialog the user asked for.
    pub dialog: Option<Dialog>,
    /// The user switched the interface language.
    pub language_changed: bool,
    /// The recent Archives or the manual entries changed and are to be stored.
    pub settings_changed: bool,
}

/// Draws the whole window for `model` and applies what the user did to it.
pub fn show(ui: &mut Ui, model: &mut Model) -> Requests {
    let mut requests = Requests {
        settings_changed: model.poll() && model.browser().is_some() && model.failure().is_none(),
        ..Requests::default()
    };
    if let Some(browser) = model.browser_mut() {
        browser.poll();
    }
    model.poll_extraction();
    model.poll_record_export();
    model.poll_update();
    model.comparison_mut().poll();
    model.query_mut().poll();
    if let Some(path) = dropped_file(ui) {
        if path.is_dir() {
            requests.settings_changed |= model.add_manual(path);
        } else {
            open(ui, model, path);
        }
    }
    shortcuts::handle(ui, model);
    let header = Panel::top("menu")
        .frame(frames::header())
        .show_separator_line(false)
        .exact_size(HEADER_HEIGHT)
        .show(ui, |ui| menu::show(ui, model, &mut requests));
    frames::header_rule(ui, header.response.rect);
    archive_change::show(ui, model);
    Panel::bottom("status")
        .frame(frames::footer())
        .exact_size(STATUS_HEIGHT)
        .show(ui, |ui| status::show(ui, model));
    Panel::left("side")
        .frame(frames::sidebar())
        .resizable(true)
        .default_size(SIDEBAR_WIDTH)
        .size_range(SIDEBAR_MIN_WIDTH..=SIDEBAR_MAX_WIDTH)
        .show(ui, |ui| {
            channels::show(ui, model);
            tree::show(ui, model);
        });
    CentralPanel::default()
        .frame(frames::canvas())
        .show(ui, |ui| work_surface(ui, model, &mut requests));
    extraction::setup(ui, model, &mut requests);
    extraction_outcome::show(ui, model);
    record_export::setup(ui, model, &mut requests);
    record_export_outcome::show(ui, model);
    licence::show(ui, model);
    query::saved_dialog(ui, model, &mut requests);
    requests
}

fn work_surface(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    work_tabs(ui, model);
    match model.work_tab() {
        WorkTab::Comparison => return comparison::show(ui, model, requests),
        WorkTab::Query => return query::show(ui, model, requests),
        WorkTab::Archive => {}
    }
    let dragging = is_dragging_file(ui);
    if dragging || model.browser().is_none() {
        empty::show(ui, model, dragging, requests);
        return;
    }
    Panel::bottom("preview")
        .frame(frames::panel().outer_margin(Margin {
            top: RESIZE_GAP,
            ..Margin::ZERO
        }))
        .resizable(true)
        .default_size(PREVIEW_HEIGHT)
        .min_size(PREVIEW_MIN_HEIGHT)
        .show(ui, |ui| preview::show(ui, model));
    frames::panel().show(ui, |ui| {
        if model.browser().is_some_and(crate::Browser::shows_records) {
            record_search::show(ui, model);
        } else {
            search::show(ui, model);
            table::show(ui, model);
        }
    });
}

const WORK_TABS: [(WorkTab, Text); 3] = [
    (
        WorkTab::Archive,
        Text::Comparison(ComparisonText::TabArchive),
    ),
    (
        WorkTab::Comparison,
        Text::Comparison(ComparisonText::TabComparison),
    ),
    (WorkTab::Query, Text::Query(QueryText::Tab)),
];

fn work_tabs(ui: &mut Ui, model: &mut Model) {
    let labels = WORK_TABS.map(|(_, text)| model.texts().get(text));
    let current = WORK_TABS
        .iter()
        .position(|(tab, _)| *tab == model.work_tab())
        .unwrap_or(0);
    if let Some((tab, _)) =
        tab::tab_bar(ui, &labels, current).and_then(|index| WORK_TABS.get(index))
    {
        model.set_work_tab(*tab);
    }
}

/// Starts opening the Archive at `path`, repainting the window once it is open.
pub fn open(ui: &Ui, model: &mut Model, path: PathBuf) {
    let ctx = ui.ctx().clone();
    model.open(path, move || ctx.request_repaint());
}

fn dropped_file(ui: &Ui) -> Option<PathBuf> {
    ui.input(|input| {
        input
            .raw
            .dropped_files
            .first()
            .map(|file| file.path().to_path_buf())
    })
}

fn is_dragging_file(ui: &Ui) -> bool {
    ui.input(|input| !input.raw.hovered_files.is_empty())
}
