use std::path::PathBuf;

use egui::{MenuBar, Ui};

use super::{Dialog, Requests};
use crate::{Language, Model, Text, Texts};

pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let texts = model.texts();
    let current = texts.language();
    let language_menu = texts.get(Text::MenuLanguage);
    let names = Language::ALL.map(|language| (language, texts.get(Text::LanguageName(language))));
    let help_menu = texts.get(Text::MenuHelp);
    let licence = texts.get(Text::FontLicence);
    let update_check = texts.get(Text::UpdateCheck);
    let mut chosen = None;
    let mut show_licence = false;
    let mut enabled = model.update_check();
    let mut reopen = None;
    MenuBar::new().ui(ui, |ui| {
        reopen = archive_menu(ui, model, requests);
        ui.menu_button(language_menu, |ui| {
            for (language, name) in names {
                if ui.radio(language == current, name).clicked() {
                    chosen = Some(language);
                }
            }
        });
        ui.menu_button(help_menu, |ui| {
            show_licence = ui.button(licence).clicked();
            ui.checkbox(&mut enabled, update_check);
        });
    });
    if enabled != model.update_check() {
        let ctx = ui.ctx().clone();
        model.switch_update_check(enabled, move || ctx.request_repaint());
        requests.settings_changed = true;
    }
    if let Some(path) = reopen {
        super::open(ui, model, path);
    }
    if show_licence {
        model.set_licence_shown(true);
    }
    if let Some(language) = chosen.filter(|&language| language != current) {
        model.set_language(language);
        requests.language_changed = true;
    }
}

fn archive_menu(ui: &mut Ui, model: &Model, requests: &mut Requests) -> Option<PathBuf> {
    let texts = model.texts();
    let mut reopen = None;
    ui.menu_button(texts.get(Text::MenuArchive), |ui| {
        for (text, dialog) in [
            (Text::ActionOpen, Dialog::OpenArchive),
            (Text::ActionAddFolder, Dialog::AddFolder),
            (Text::ActionAddArchive, Dialog::AddArchive),
        ] {
            if ui.button(texts.get(text)).clicked() {
                requests.dialog = Some(dialog);
            }
        }
        ui.separator();
        ui.menu_button(texts.get(Text::MenuRecent), |ui| {
            reopen = recent_menu(ui, texts, model);
        });
    });
    reopen
}

fn recent_menu(ui: &mut Ui, texts: &Texts, model: &Model) -> Option<PathBuf> {
    if model.recent().is_empty() {
        ui.weak(texts.get(Text::RecentNone));
    }
    let mut reopen = None;
    for path in model.recent() {
        if ui.button(path.display().to_string()).clicked() {
            reopen = Some(path.clone());
        }
    }
    reopen
}
