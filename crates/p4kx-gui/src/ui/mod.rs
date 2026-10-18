//! The egui code that draws a [`Model`] and turns clicks into changes of it.

mod channels;
mod extraction;
mod extraction_outcome;
mod licence;
mod menu;
mod preview;
mod search;
mod status;
mod table;
mod tree;

use std::path::PathBuf;

use egui::{CentralPanel, Panel, Ui};

use crate::{Model, Text};

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
    if let Some(path) = dropped_file(ui) {
        if path.is_dir() {
            requests.settings_changed |= model.add_manual(path);
        } else {
            open(ui, model, path);
        }
    }
    Panel::top("menu").show(ui, |ui| menu::show(ui, model, &mut requests));
    Panel::bottom("status").show(ui, |ui| status::show(ui, model));
    Panel::left("side")
        .resizable(true)
        .default_size(280.0)
        .show(ui, |ui| {
            channels::show(ui, model);
            ui.separator();
            tree::show(ui, model);
        });
    CentralPanel::default().show(ui, |ui| {
        if is_dragging_file(ui) || model.browser().is_none() {
            ui.centered_and_justified(|ui| ui.label(model.texts().get(Text::DropHint)));
        } else {
            search::show(ui, model);
            extraction::actions(ui, model);
            Panel::bottom("preview")
                .resizable(true)
                .default_size(300.0)
                .show(ui, |ui| preview::show(ui, model));
            table::show(ui, model);
        }
    });
    extraction::setup(ui, model, &mut requests);
    extraction_outcome::show(ui, model);
    licence::show(ui, model);
    requests
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

fn row_height(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y
}
