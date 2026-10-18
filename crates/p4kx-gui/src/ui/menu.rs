use std::path::PathBuf;

use egui::{Button, MenuBar, Response, Role, Ui, WidgetInfo};

use super::{Dialog, Requests, shortcuts};
use crate::theme::desktop::{
    BRAND_PADDING_RIGHT, DROPDOWN_ITEM_PADDING_X, DROPDOWN_MIN_WIDTH, MENU_LINK_PADDING_X,
    MENU_LINK_SIZE,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, TOUCH_TARGET, TRACKING_DISPLAY, Weight, font};
use crate::widgets::indicator::brand;
use crate::widgets::text::caps;
use crate::widgets::toggle;
use crate::{ExtractionScope, ExtractionText, Language, Model, Text, Texts};

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
    let mut pick = None;
    ui.horizontal_centered(|ui| {
        brand(ui, &app);
        ui.add_space(BRAND_PADDING_RIGHT);
        ui.spacing_mut().interact_size.y = TOUCH_TARGET;
        MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().button_padding.x = MENU_LINK_PADDING_X;
            pick = archive_menu(ui, model, requests);
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
    if let Some(pick) = pick {
        apply(ui, model, pick);
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
fn entry(text: &str) -> egui::RichText {
    egui::RichText::new(text).font(font(FS_SM, Weight::Regular))
}

/// Returns a dropdown entry with its shortcut hint right-aligned in the muted tint.
fn shortcut_entry(texts: &Texts, text: Text, hint: Text) -> Button<'static> {
    let hint = egui::RichText::new(texts.get(hint))
        .font(font(FS_XS, Weight::Regular))
        .color(palette::GRAY_2_TEXT);
    Button::new(entry(&texts.get(text))).shortcut_text(hint)
}

fn dropdown(ui: &mut Ui) {
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

/// What the user picked in the Archive menu besides a dialog.
enum Pick {
    Reopen(PathBuf),
    Reload,
    ExtractSelected,
}

fn apply(ui: &Ui, model: &mut Model, pick: Pick) {
    match pick {
        Pick::Reopen(path) => super::open(ui, model, path),
        Pick::Reload => {
            let ctx = ui.ctx().clone();
            model.reload(move || ctx.request_repaint());
        }
        Pick::ExtractSelected => model.begin_extraction(ExtractionScope::Selected),
    }
}

fn archive_menu(ui: &mut Ui, model: &Model, requests: &mut Requests) -> Option<Pick> {
    let texts = model.texts();
    let mut pick = None;
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
        ui.menu_button(entry(&texts.get(Text::MenuRecent)), |ui| {
            dropdown(ui);
            if let Some(path) = recent_menu(ui, texts, model) {
                pick = Some(Pick::Reopen(path));
            }
        });
        ui.separator();
        let reload = shortcut_entry(texts, Text::ActionReload, Text::ShortcutReload);
        if ui
            .add_enabled(shortcuts::reload_enabled(model), reload)
            .clicked()
        {
            pick = Some(Pick::Reload);
        }
        let extract = shortcut_entry(
            texts,
            ExtractionText::ActionSelected.into(),
            Text::ShortcutExtractSelected,
        );
        if ui
            .add_enabled(shortcuts::extract_selected_enabled(model), extract)
            .clicked()
        {
            pick = Some(Pick::ExtractSelected);
        }
    });
    label(&response.response, &title);
    pick
}

fn recent_menu(ui: &mut Ui, texts: &Texts, model: &Model) -> Option<PathBuf> {
    if model.recent().is_empty() {
        ui.label(entry(&texts.get(Text::RecentNone)).color(palette::GRAY_2_TEXT));
    }
    let mut reopen = None;
    for path in model.recent() {
        if ui.button(entry(&path.display().to_string())).clicked() {
            reopen = Some(path.clone());
        }
    }
    reopen
}
