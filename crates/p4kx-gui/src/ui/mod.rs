// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The egui code that draws a [`Model`] and turns clicks into changes of it.

mod about;
mod archive_change;
mod archive_issues;
mod archive_key;
mod archive_notice;
mod archive_picker;
mod assembly_export;
mod assembly_export_outcome;
mod assembly_export_progress;
mod audio_controls;
mod audio_preview;
mod catalog_outcome;
mod channels;
mod character_choices;
mod character_chooser;
mod character_export;
mod character_report;
mod character_toolbar;
mod character_view;
mod characters;
mod comparison;
mod comparison_diff;
mod comparison_extraction;
mod comparison_list;
mod comparison_lists;
mod comparison_rows;
mod comparison_schema;
mod comparison_summary;
mod comparison_table;
mod container_note;
mod content_search;
mod content_search_notes;
mod content_search_rows;
mod dataforge_summary;
mod diff_rows;
mod empty;
mod entities;
mod entity_list;
mod entity_loadout;
mod entity_replace;
mod entity_view;
mod entry_status;
mod extraction;
mod extraction_outcome;
mod field_rows;
mod game_running;
mod hex_view;
mod image_preview;
mod licence;
mod list_footer;
mod localization;
mod localization_laid;
mod localization_table;
mod menu;
mod model_export;
mod model_export_animations;
mod model_export_outcome;
mod model_info;
mod model_info_rows;
mod model_playback;
mod model_preview;
mod model_view;
mod moved_path;
mod preview;
mod preview_bar;
mod preview_tab;
mod query;
mod query_editor;
mod query_table;
mod record_export;
mod record_export_outcome;
mod record_fields;
mod record_incoming;
mod record_key;
mod record_lines;
mod record_list;
mod record_preview;
mod record_results;
mod record_rows;
mod record_search;
mod record_tree;
mod record_value;
mod search;
mod shortcuts;
mod sound_grid;
mod soundbank_preview;
mod soundbank_table;
mod sounds;
mod sounds_export;
mod sounds_outcome;
mod sounds_toolbar;
mod sounds_triggers;
mod sounds_variants;
mod status;
mod table;
mod table_name;
mod tags;
mod tags_table;
mod tree;
mod viewer_input;
mod xml_preview;
mod xml_view;

use std::path::PathBuf;

use egui::{CentralPanel, Margin, Panel, Ui};

use crate::theme::desktop::{
    HEADER_HEIGHT, PREVIEW_HEIGHT, PREVIEW_MIN_HEIGHT, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
    SIDEBAR_WIDTH, STATUS_HEIGHT,
};
use crate::theme::model_metrics::PREVIEW_MODEL_HEIGHT;
use crate::theme::sound_metrics::PREVIEW_SOUNDBANK_HEIGHT;
use crate::widgets::{frames, tab};

const RESIZE_GAP: i8 = 6;
use crate::{
    CharacterText, ComparisonSide, ComparisonText, ContentSearchText, EntityText, LocalizationText,
    Model, PreviewTab, QueryExportFormat, QueryText, SoundsText, TagsText, Text, WorkTab,
};

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
    /// Picks the file the Basetool catalog is written to.
    CatalogFile,
    /// Picks the GLB file a model is exported to.
    ModelFile,
    /// Picks the GLB file an Entity is exported to.
    AssemblyFile,
    /// Picks the folder of a sound export.
    SoundsFolder,
    /// Picks the CHF file of a head.
    CharacterChf,
    /// Picks the GLB file a Character is exported to.
    CharacterFile,
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
    poll_background(model);
    release_assembly(ui, model);
    poll_search(ui, model);
    if let Some(path) = dropped_file(ui) {
        if path.is_dir() {
            requests.settings_changed |= model.add_manual(path);
        } else {
            open(ui, model, path);
        }
    }
    shortcuts::handle(ui, model);
    record_preview::handle_shortcuts(ui, model);
    let header = Panel::top("menu")
        .frame(frames::header())
        .show_separator_line(false)
        .exact_size(HEADER_HEIGHT)
        .show(ui, |ui| menu::show(ui, model, &mut requests));
    frames::header_rule(ui, header.response.rect);
    game_running::show(ui, model);
    archive_change::show(ui, model);
    archive_issues::show(ui, model);
    archive_notice::show(ui, model);
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
    model_export::setup(ui, model, &mut requests);
    model_export_outcome::show(ui, model);
    assembly_export::setup(ui, model, &mut requests);
    assembly_export_outcome::show(ui, model);
    character_export::show(ui, model);
    catalog_outcome::show(ui, model);
    sounds_export::setup(ui, model, &mut requests);
    sounds_outcome::show(ui, model);
    licence::show(ui, model);
    about::show(ui, model);
    archive_key::show(ui, model);
    query::saved_dialog(ui, model, &mut requests);
    requests
}

/// Frees the Assembly buffers on the graphics device once no Assembly is on screen: another tab
/// is shown, or the selection or the Archive changed.
fn release_assembly(ui: &Ui, model: &Model) {
    let crate::ViewerSupport::Ready(report) = model.viewer_support() else {
        return;
    };
    let shown = model.work_tab() == WorkTab::Entities
        && model
            .entities()
            .loadout()
            .is_some_and(|session| session.assembly().scene().is_some());
    if !shown && report.holds_assembly() {
        ui.painter().add(crate::viewer::release_callback());
    }
}

/// Takes the result of a content search and shows a hit that waited for its data.
fn poll_search(ui: &Ui, model: &mut Model) {
    model.content_search_mut().poll();
    let ctx = ui.ctx().clone();
    model.apply_pending_hit(move || ctx.request_repaint());
}

fn poll_background(model: &mut Model) {
    if let Some(browser) = model.browser_mut() {
        browser.poll();
    }
    model.poll_extraction();
    model.poll_record_export();
    model.poll_model_export();
    model.poll_assembly_export();
    model.entities_mut().poll();
    model.characters_mut().poll();
    model.poll_catalog();
    model.poll_update();
    model.comparison_mut().poll();
    model.poll_key_check();
    model.poll_key_request();
    model.query_mut().poll();
    model.localization_mut().poll();
    model.tags_mut().poll();
    model.poll_sounds();
}

fn work_surface(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    work_tabs(ui, model);
    frames::inset().show(ui, |ui| work_panels(ui, model, requests));
}

fn work_panels(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    match model.work_tab() {
        WorkTab::Comparison => return comparison::show(ui, model, requests),
        WorkTab::Query => return query::show(ui, model, requests),
        WorkTab::Localization => return localization::show(ui, model),
        WorkTab::Tags => return tags::show(ui, model),
        WorkTab::Search => return content_search::show(ui, model),
        WorkTab::Entities => return entities::show(ui, model),
        WorkTab::Sounds => return sounds::show(ui, model),
        WorkTab::Characters => return characters::show(ui, model, requests),
        WorkTab::Archive => {}
    }
    let dragging = is_dragging_file(ui);
    if dragging || model.browser().is_none() {
        empty::show(ui, model, dragging, requests);
        return;
    }
    let soundbank = model.preview_tab() == PreviewTab::Audio
        && model
            .browser()
            .is_some_and(|browser| browser.soundbank().is_some());
    let (salt, height) = if soundbank {
        ("preview-soundbank", PREVIEW_SOUNDBANK_HEIGHT)
    } else if model.preview_tab() == PreviewTab::Model {
        ("preview-model", PREVIEW_MODEL_HEIGHT)
    } else {
        ("preview", PREVIEW_HEIGHT)
    };
    Panel::bottom(salt)
        .frame(frames::panel().outer_margin(Margin {
            top: RESIZE_GAP,
            ..Margin::ZERO
        }))
        .resizable(true)
        .default_size(height)
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

const WORK_TABS: [(WorkTab, Text); 9] = [
    (
        WorkTab::Archive,
        Text::Comparison(ComparisonText::TabArchive),
    ),
    (
        WorkTab::Comparison,
        Text::Comparison(ComparisonText::TabComparison),
    ),
    (WorkTab::Query, Text::Query(QueryText::Tab)),
    (
        WorkTab::Localization,
        Text::Localization(LocalizationText::Tab),
    ),
    (WorkTab::Tags, Text::Tags(TagsText::Tab)),
    (WorkTab::Search, Text::ContentSearch(ContentSearchText::Tab)),
    (WorkTab::Entities, Text::Entity(EntityText::Tab)),
    (WorkTab::Sounds, Text::Sounds(SoundsText::Tab)),
    (WorkTab::Characters, Text::Character(CharacterText::Tab)),
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
