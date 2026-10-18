use std::path::PathBuf;

use egui::{MenuBar, Response, Role, Ui, WidgetInfo};

use super::{Dialog, Requests};
use crate::theme::desktop::{
    BRAND_PADDING_RIGHT, DROPDOWN_ITEM_PADDING_X, DROPDOWN_MIN_WIDTH, MENU_LINK_PADDING_X,
    MENU_LINK_SIZE,
};
use crate::theme::tokens::{FS_SM, TOUCH_TARGET, TRACKING_DISPLAY, Weight, font};
use crate::widgets::indicator::brand;
use crate::widgets::text::caps;
use crate::widgets::toggle;
use crate::{Language, Model, Text, Texts};

/// Shows the header: the brand and the menus.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let texts = model.texts();
    let current = texts.language();
    let app = texts.get(Text::AppTitle);
    let language_menu = texts.get(Text::MenuLanguage);
    let names = Language::ALL.map(|language| (language, texts.get(Text::LanguageName(language))));
    let help_menu = texts.get(Text::MenuHelp);
    let licence = texts.get(Text::FontLicence);
    let update_check = texts.get(Text::UpdateCheck);
    let mut chosen = None;
    let mut show_licence = false;
    let mut enabled = model.update_check();
    let mut reopen = None;
    ui.horizontal_centered(|ui| {
        brand(ui, &app);
        ui.add_space(BRAND_PADDING_RIGHT);
        ui.spacing_mut().interact_size.y = TOUCH_TARGET;
        MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().button_padding.x = MENU_LINK_PADDING_X;
            reopen = archive_menu(ui, model, requests);
            let response = ui.menu_button(link(&language_menu), |ui| {
                dropdown(ui);
                for (language, name) in &names {
                    let clicked = indented(ui, |ui| {
                        toggle::radio(ui, *language == current, name, true).clicked()
                    });
                    if clicked {
                        chosen = Some(*language);
                    }
                }
            });
            label(&response.response, &language_menu);
            let response = ui.menu_button(link(&help_menu), |ui| {
                dropdown(ui);
                show_licence = ui.button(entry(&licence)).clicked();
                ui.separator();
                indented(ui, |ui| toggle::checkbox(ui, &mut enabled, &update_check));
            });
            label(&response.response, &help_menu);
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

fn link(text: &str) -> egui::RichText {
    caps(text, MENU_LINK_SIZE, Weight::Bold, TRACKING_DISPLAY)
}

/// Returns a `.krt-combobox__option` label: 14.4 points Regular, as written.
pub(super) fn entry(text: &str) -> egui::RichText {
    egui::RichText::new(text).font(font(FS_SM, Weight::Regular))
}

pub(super) fn dropdown(ui: &mut Ui) {
    ui.set_min_width(DROPDOWN_MIN_WIDTH);
    ui.spacing_mut().button_padding.x = DROPDOWN_ITEM_PADDING_X;
}

fn indented<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_space(DROPDOWN_ITEM_PADDING_X - ui.spacing().item_spacing.x);
        add(ui)
    })
    .inner
}

fn label(response: &Response, text: &str) {
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, text));
}

fn archive_menu(ui: &mut Ui, model: &Model, requests: &mut Requests) -> Option<PathBuf> {
    let texts = model.texts();
    let mut reopen = None;
    let title = texts.get(Text::MenuArchive);
    let response = ui.menu_button(link(&title), |ui| {
        dropdown(ui);
        for (text, dialog) in [
            (Text::ActionOpen, Dialog::OpenArchive),
            (Text::ActionAddFolder, Dialog::AddFolder),
            (Text::ActionAddArchive, Dialog::AddArchive),
        ] {
            if ui.button(entry(&texts.get(text))).clicked() {
                requests.dialog = Some(dialog);
            }
        }
        ui.separator();
        ui.menu_button(entry(&texts.get(Text::MenuRecent)), |ui| {
            dropdown(ui);
            reopen = recent_menu(ui, texts, model);
        });
    });
    label(&response.response, &title);
    reopen
}

fn recent_menu(ui: &mut Ui, texts: &Texts, model: &Model) -> Option<PathBuf> {
    if model.recent().is_empty() {
        ui.label(entry(&texts.get(Text::RecentNone)).color(crate::theme::palette::GRAY_2_TEXT));
    }
    let mut reopen = None;
    for path in model.recent() {
        if ui.button(entry(&path.display().to_string())).clicked() {
            reopen = Some(path.clone());
        }
    }
    reopen
}
