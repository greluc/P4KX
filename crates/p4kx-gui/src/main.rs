//! `p4kx-gui`, the P4K-Extractor window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use p4kx_channels::SearchPaths;
use p4kx_gui::{App, ChannelList, Language, Model, RecordTextChoice, Text, theme, window};
use p4kx_settings::{
    KEY_VARIABLE, KeyChoiceError, KeyStore, Settings, choose_key, default_settings_file,
};

/// Gives the model the key of the environment variable or the profile. A variable that holds no
/// valid key is ignored in favour of the stored key, and so is a stored key that cannot be read
/// in favour of none; the dialog says so in both cases.
fn set_up_key(model: &mut Model) {
    let key_store = std::env::var_os("P4KX_KEY_FILE")
        .filter(|file| !file.is_empty())
        .map_or_else(KeyStore::in_profile, |file| Some(KeyStore::new(file)));
    let variable = std::env::var(KEY_VARIABLE).ok();
    let (chosen, variable_invalid) = match choose_key(None, variable.as_deref(), key_store.as_ref())
    {
        Ok(chosen) => (chosen, false),
        Err(KeyChoiceError::Environment(_)) => (
            choose_key(None, None, key_store.as_ref()).ok().flatten(),
            true,
        ),
        Err(KeyChoiceError::Flag(_) | KeyChoiceError::Profile(_)) => (None, false),
    };
    model.set_key_store(key_store);
    model.set_key_variable_invalid(variable_invalid);
    model.set_archive_key(chosen.map(|chosen| chosen.key));
}

fn main() -> Result<()> {
    p4kx_extraction::protect_libraries();
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
    model.set_extraction_textures(settings.extraction_textures);
    model.set_extraction_audio(settings.extraction_audio);
    model.set_update_check(settings.update_check);
    model.watch_game(true);
    model.query_mut().set_saved(settings.queries.clone());
    model.set_record_text_choice(
        settings
            .record_language
            .as_deref()
            .and_then(RecordTextChoice::from_code),
    );
    set_up_key(&mut model);
    let title = model.texts().get(Text::AppTitle);
    let options = window::native_options(window::viewport(settings.window, &title));
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
