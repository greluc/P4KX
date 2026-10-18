use egui::{MenuBar, Ui};

use super::Requests;
use crate::{Language, Model, Text};

pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let texts = model.texts();
    let current = texts.language();
    let archive_menu = texts.get(Text::MenuArchive);
    let open = texts.get(Text::ActionOpen);
    let language_menu = texts.get(Text::MenuLanguage);
    let names = Language::ALL.map(|language| (language, texts.get(Text::LanguageName(language))));
    let help_menu = texts.get(Text::MenuHelp);
    let licence = texts.get(Text::FontLicence);
    let mut chosen = None;
    let mut show_licence = false;
    MenuBar::new().ui(ui, |ui| {
        ui.menu_button(archive_menu, |ui| {
            if ui.button(open).clicked() {
                requests.open_dialog = true;
            }
        });
        ui.menu_button(language_menu, |ui| {
            for (language, name) in names {
                if ui.radio(language == current, name).clicked() {
                    chosen = Some(language);
                }
            }
        });
        ui.menu_button(help_menu, |ui| {
            show_licence = ui.button(licence).clicked();
        });
    });
    if show_licence {
        model.set_licence_shown(true);
    }
    if let Some(language) = chosen.filter(|&language| language != current) {
        model.set_language(language);
        requests.language_changed = true;
    }
}
