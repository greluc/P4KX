use std::path::PathBuf;

use egui::{Button, Color32, MenuBar, Response, Role, Ui, WidgetInfo};

use super::{Dialog, Requests, shortcuts};
use crate::texts::language_name;
use crate::theme::desktop::{
    BRAND_PADDING_RIGHT, DROPDOWN_ITEM_PADDING_X, DROPDOWN_MIN_WIDTH, MENU_LINK_PADDING_X,
    MENU_LINK_SIZE,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, TOUCH_TARGET, TRACKING_DISPLAY, Weight, font};
use crate::widgets::indicator::brand;
use crate::widgets::text::caps;
use crate::widgets::toggle;
use crate::{
    CatalogText, ExtractionScope, ExtractionText, Language, Model, RecordExportText,
    RecordLanguageText, RecordTextChoice, Text, Texts,
};

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
    let mut text_choice = None;
    let mut show_licence = false;
    let mut enabled = model.update_check();
    let mut pick = None;
    ui.horizontal_centered(|ui| {
        brand(ui, &app);
        ui.add_space(BRAND_PADDING_RIGHT);
        ui.spacing_mut().interact_size.y = TOUCH_TARGET;
        MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().button_padding.x = MENU_LINK_PADDING_X;
            ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::TRANSPARENT;
            pick = archive_menu(ui, model, requests);
            if export_menu(ui, model, requests) {
                pick = Some(Pick::RecordExport);
            }
            text_choice = record_text_menu(ui, model);
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
        switch_update_check(ui, model, requests, enabled);
    }
    if let Some(pick) = pick {
        apply(ui, model, pick);
    }
    choose_record_texts(model, requests, text_choice);
    if show_licence {
        model.set_licence_shown(true);
    }
    if let Some(language) = chosen.filter(|&language| language != current) {
        model.set_language(language);
        requests.language_changed = true;
    }
}

fn switch_update_check(ui: &Ui, model: &mut Model, requests: &mut Requests, enabled: bool) {
    let ctx = ui.ctx().clone();
    model.switch_update_check(enabled, move || ctx.request_repaint());
    requests.settings_changed = true;
}

fn choose_record_texts(
    model: &mut Model,
    requests: &mut Requests,
    choice: Option<RecordTextChoice>,
) {
    if choice.is_some() {
        model.set_record_text_choice(choice);
        requests.settings_changed = true;
    }
}

fn link(text: &str) -> egui::RichText {
    caps(text, MENU_LINK_SIZE, Weight::Bold, TRACKING_DISPLAY)
}

/// Returns a `.krt-combobox__option` label: 14.4 points Regular, as written.
pub(super) fn entry(text: &str) -> egui::RichText {
    egui::RichText::new(text).font(font(FS_SM, Weight::Regular))
}

/// Returns a dropdown entry with its shortcut hint right-aligned in the muted tint.
fn shortcut_entry(texts: &Texts, text: Text, hint: Text) -> Button<'static> {
    let hint = egui::RichText::new(texts.get(hint))
        .font(font(FS_XS, Weight::Regular))
        .color(palette::GRAY_2_TEXT);
    Button::new(entry(&texts.get(text))).shortcut_text(hint)
}

pub(super) fn dropdown(ui: &mut Ui) {
    ui.set_min_width(DROPDOWN_MIN_WIDTH);
    ui.spacing_mut().button_padding.x = DROPDOWN_ITEM_PADDING_X;
    ui.visuals_mut().widgets.hovered.weak_bg_fill = palette::GRAY_3;
}

fn indented<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_space(DROPDOWN_ITEM_PADDING_X - ui.spacing().item_spacing.x);
        add(ui)
    })
    .inner
}

fn label(response: &Response, text: &str) {
    response.widget_info(|| WidgetInfo::labeled(Role::Button, response.enabled(), text));
}

/// What the user picked in the Archive menu besides a dialog.
enum Pick {
    Reopen(PathBuf),
    Reload,
    ExtractSelected,
    RecordExport,
}

fn apply(ui: &Ui, model: &mut Model, pick: Pick) {
    match pick {
        Pick::Reopen(path) => super::open(ui, model, path),
        Pick::Reload => {
            let ctx = ui.ctx().clone();
            model.reload(move || ctx.request_repaint());
        }
        Pick::ExtractSelected => model.begin_extraction(ExtractionScope::Selected),
        Pick::RecordExport => model.begin_record_export(),
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

/// Shows the Export menu; returns whether Record export was picked, and asks for the catalog file
/// when Basetool catalog was picked.
fn export_menu(ui: &mut Ui, model: &Model, requests: &mut Requests) -> bool {
    let texts = model.texts();
    let title = texts.get(RecordExportText::Menu);
    let mut picked = false;
    let response = ui.menu_button(link(&title), |ui| {
        dropdown(ui);
        let record_export = Button::new(entry(&texts.get(RecordExportText::Action)));
        picked = ui
            .add_enabled(model.record_export_available(), record_export)
            .clicked();
        let catalog = Button::new(entry(&texts.get(CatalogText::Action)));
        if ui.add_enabled(model.catalog_available(), catalog).clicked() {
            requests.dialog = Some(Dialog::CatalogFile);
        }
    });
    label(&response.response, &title);
    picked
}

/// Shows the menu that chooses the language of the texts next to localisation keys in Record
/// previews, one radio per language the open Archive holds; returns the choice that was made.
fn record_text_menu(ui: &mut Ui, model: &Model) -> Option<RecordTextChoice> {
    let texts = model.texts();
    let title = texts.get(RecordLanguageText::Menu);
    let languages = model.record_text_languages();
    let current = model.record_text_language();
    let keys_only = model.record_text_choice() == Some(RecordTextChoice::KeysOnly);
    let unavailable = model.record_text_unavailable();
    let mut picked = None;
    let response = ui
        .add_enabled_ui(!languages.is_empty(), |ui| {
            ui.menu_button(link(&title), |ui| {
                dropdown(ui);
                let keys_only_name = texts.get(RecordLanguageText::KeysOnly);
                if indented(ui, |ui| {
                    toggle::radio(ui, keys_only, &keys_only_name, true).clicked()
                }) {
                    picked = Some(RecordTextChoice::KeysOnly);
                }
                for &language in languages {
                    let name = language_name(texts, language);
                    if indented(ui, |ui| {
                        toggle::radio(ui, current == Some(language), &name, true).clicked()
                    }) {
                        picked = Some(RecordTextChoice::Language(language));
                    }
                }
                if let Some(language) = unavailable {
                    ui.separator();
                    let args = [("language", language_name(texts, language).into())];
                    let note = texts.format(RecordLanguageText::Unavailable, &args);
                    ui.scope(|ui| {
                        ui.set_max_width(DROPDOWN_MIN_WIDTH);
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(DROPDOWN_ITEM_PADDING_X - ui.spacing().item_spacing.x);
                            ui.label(entry(&note).color(palette::GRAY_2_TEXT));
                        });
                    });
                }
            })
        })
        .inner;
    label(&response.response, &title);
    picked
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
