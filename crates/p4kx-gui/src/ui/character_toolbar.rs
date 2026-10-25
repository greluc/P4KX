// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Layout, Ui};
use p4kx_settings::ModelMaterials;

use super::search::eyebrow;
use crate::theme::desktop::{AUDIO_CONTROL_GAP, TOOLBAR_PADDING};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::tab;
use crate::{CHARACTER_LODS, CharacterText as T, CharactersTab, EntityText, Model, Texts};

const MATERIALS: [ModelMaterials; 2] = [ModelMaterials::Colours, ModelMaterials::Textures];

enum Ask {
    Lod(u32),
    Materials(ModelMaterials),
    Export,
}

/// Shows the toolbar of the Characters tab: the level of detail, how the materials are written
/// and the export, which opens its setup.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let ask = {
        let (tab, texts) = model.characters_with_texts();
        let tab: &CharactersTab = tab;
        ui.spacing_mut().item_spacing = egui::vec2(AUDIO_CONTROL_GAP, TOOLBAR_PADDING / 2.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let exported = export(ui, tab, texts);
                let chosen = ui
                    .with_layout(Layout::left_to_right(Align::Center), |ui| {
                        settings(ui, tab, texts)
                    })
                    .inner;
                exported.or(chosen)
            })
            .inner
        })
        .inner
    };
    match ask {
        Some(Ask::Lod(level)) => model.characters_mut().set_lod(level),
        Some(Ask::Materials(materials)) => model.characters_mut().set_materials(materials),
        Some(Ask::Export) => {
            model.begin_character_export();
        }
        None => {}
    }
}

fn export(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    let action = texts.get(T::ExportAction);
    button::button(ui, Kind::Cta, Size::Normal, &action, tab.export_available())
        .clicked()
        .then_some(Ask::Export)
}

fn settings(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    let detail = detail(ui, tab, texts);
    ui.add_space(AUDIO_CONTROL_GAP);
    materials(ui, tab, texts).or(detail)
}

fn detail(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    ui.label(eyebrow(&texts.get(EntityText::ViewLod)));
    let levels = CHARACTER_LODS.map(|level| level.to_string());
    let active = CHARACTER_LODS.iter().position(|level| *level == tab.lod());
    tab::segments(ui, &levels, active)
        .and_then(|at| CHARACTER_LODS.get(at))
        .map(|level| Ask::Lod(*level))
}

fn materials(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    ui.label(eyebrow(&texts.get(EntityText::ViewMaterials)));
    let names = MATERIALS.map(|materials| {
        texts.get(match materials {
            ModelMaterials::Colours => EntityText::ViewColours,
            ModelMaterials::Textures => EntityText::ViewTextures,
        })
    });
    let active = MATERIALS
        .iter()
        .position(|materials| *materials == tab.materials());
    tab::segments(ui, &names, active)
        .and_then(|at| MATERIALS.get(at))
        .map(|materials| Ask::Materials(*materials))
}
