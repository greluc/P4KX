use std::path::Path;

use egui::Ui;
use p4kx_channels::Channel;

use crate::{Model, Text, Texts};

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let current = current_archive(model);
    let channels = model.channels().detected();
    let mut picked = None;
    ui.strong(texts.get(Text::ChannelsHeading));
    if channels.is_empty() {
        ui.weak(texts.get(Text::ChannelsNone));
    }
    for channel in channels {
        if show_row(ui, texts, channel, current) && current != Some(channel.archive()) {
            picked = Some(channel.archive().to_path_buf());
        }
    }
    if let Some(path) = picked {
        super::open(ui, model, path);
    }
}

fn show_row(ui: &mut Ui, texts: &Texts, channel: &Channel, current: Option<&Path>) -> bool {
    let build = super::status::build_label(texts, channel.build());
    ui.horizontal(|ui| {
        let selected = current == Some(channel.archive());
        let clicked = ui
            .selectable_label(selected, channel.name())
            .on_hover_text(channel.archive().display().to_string())
            .clicked();
        ui.weak(build);
        clicked
    })
    .inner
}

fn current_archive(model: &Model) -> Option<&Path> {
    model.opening_path().or_else(|| {
        model
            .browser()
            .map(|browser| browser.index().archive().path())
    })
}
