//! `p4kx-gui`, the P4K-Extractor window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use p4kx_channels::SearchPaths;
use p4kx_gui::{App, ChannelList, Language, Model, Text, theme, window};
use p4kx_settings::{Settings, default_settings_file};

fn main() -> Result<()> {
    let settings_file = default_settings_file();
    let settings = settings_file
        .as_deref()
        .and_then(|file| Settings::load(file).ok())
        .unwrap_or_default();
    let mut model = Model::new(Language::resolve(settings.language.as_deref()));
    model.set_channels(ChannelList::new(
        SearchPaths::standard(),
        settings.channels.clone(),
    ));
    model.set_recent(settings.recent.clone());
    model.set_extraction_folder(settings.extraction_folder.clone());
    model.set_extraction_conversion(settings.extraction_conversion);
    model.set_update_check(settings.update_check);
    let title = model.texts().get(Text::AppTitle);
    let options = eframe::NativeOptions {
        viewport: window::viewport(settings.window, &title),
        ..eframe::NativeOptions::default()
    };
    let archive = std::env::args_os().nth(1).map(PathBuf::from);
    eframe::run_native(
        &title,
        options,
        Box::new(move |creation| {
            theme::apply(&creation.egui_ctx);
            #[cfg(windows)]
            p4kx_gui::dwm::tint(creation);
            let mut model = model;
            let ctx = creation.egui_ctx.clone();
            model.check_for_update(move || ctx.request_repaint());
            if let Some(path) = archive {
                let ctx = creation.egui_ctx.clone();
                model.open(path, move || ctx.request_repaint());
            }
            Ok(Box::new(App::new(model, settings_file, settings.window)))
        }),
    )
    .map_err(|error| anyhow!("cannot show the window: {error}"))
}
