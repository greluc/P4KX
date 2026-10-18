//! The egui code that draws a [`Model`] and turns clicks into changes of it.

mod licence;
mod menu;
mod status;
mod table;
mod tree;

use std::path::PathBuf;

use egui::{CentralPanel, Panel, Ui};

use crate::{Model, Text};

/// What the window asks of its host after a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requests {
    /// The user asked for the open dialog.
    pub open_dialog: bool,
    /// The user switched the interface language.
    pub language_changed: bool,
}

/// Draws the whole window for `model` and applies what the user did to it.
pub fn show(ui: &mut Ui, model: &mut Model) -> Requests {
    model.poll();
    if let Some(path) = dropped_file(ui) {
        open(ui, model, path);
    }
    let mut requests = Requests::default();
    Panel::top("menu").show(ui, |ui| menu::show(ui, model, &mut requests));
    Panel::bottom("status").show(ui, |ui| status::show(ui, model));
    Panel::left("tree")
        .resizable(true)
        .default_size(280.0)
        .show(ui, |ui| tree::show(ui, model));
    CentralPanel::default().show(ui, |ui| {
        if is_dragging_file(ui) || model.browser().is_none() {
            ui.centered_and_justified(|ui| ui.label(model.texts().get(Text::DropHint)));
        } else {
            table::show(ui, model);
        }
    });
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
