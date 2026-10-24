// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Layout, Ui};
use p4kx_settings::ModelMaterials;

use super::extraction::{foot, group_label, path_field, refusal_notice};
use super::{Dialog, Requests};
use crate::theme::desktop::MODAL_GROUP_GAP;
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone};
use crate::widgets::text::plain;
use crate::widgets::{tab, toggle};
use crate::{
    ASSEMBLY_LODS, ASSEMBLY_MATERIALS, ASSEMBLY_TEXTURES_AVAILABLE, AssemblyExportSetup,
    AssemblyExportText as T, ExtractionText, MODEL_TEXTURE_MIPS, Model, Text, Texts,
};

enum Action {
    Submit,
    Close,
    ChooseFile,
}

/// Shows the Export of an Entity as GLB setup while it is open and applies what the user chose.
pub(super) fn setup(ui: &Ui, model: &mut Model, requests: &mut Requests) {
    let Some((setup, texts)) = model.assembly_export_setup_with_texts() else {
        return;
    };
    let title = texts.get(T::SetupTitle);
    let close_dialog = texts.get(Text::CloseDialog);
    let shown = frames::modal(frames::modal_id("assembly-export-setup"), Tone::Normal).show(
        ui.ctx(),
        |ui| {
            ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
            let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
            let action = choices(ui, texts, setup);
            if closed { Some(Action::Close) } else { action }
        },
    );
    frames::modal_decoration(
        ui.ctx(),
        shown.response.layer_id,
        shown.response.rect,
        Tone::Normal,
    );
    let close = shown.should_close();
    match shown.inner {
        Some(Action::Submit) => {
            let ctx = ui.ctx().clone();
            requests.settings_changed |=
                model.submit_assembly_export(move || ctx.request_repaint());
        }
        Some(Action::Close) => model.close_assembly_export_setup(),
        Some(Action::ChooseFile) => requests.dialog = Some(Dialog::AssemblyFile),
        None if close => model.close_assembly_export_setup(),
        None => {}
    }
}

fn choices(ui: &mut Ui, texts: &Texts, setup: &mut AssemblyExportSetup) -> Option<Action> {
    let mut action = None;
    let entity = texts.format(T::Entity, &[("name", setup.name().to_owned().into())]);
    ui.label(plain(entity, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT));
    let (excluded, replaced) = (
        setup.changes().exclusions().len(),
        setup.changes().replacements().len(),
    );
    let changes = if excluded + replaced == 0 {
        texts.get(T::ChangesNone)
    } else {
        texts.format(
            T::Changes,
            &[
                ("excluded", excluded.to_string().into()),
                ("replaced", replaced.to_string().into()),
            ],
        )
    };
    ui.label(plain(changes, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT));
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::LodHeading));
    let labels = ASSEMBLY_LODS.map(|level| texts.format(T::Lod, &[("level", level.into())]));
    let active = ASSEMBLY_LODS.iter().position(|level| *level == setup.lod());
    let clicked = ui.horizontal(|ui| tab::segments(ui, &labels, active)).inner;
    if let Some(level) = clicked.and_then(|at| ASSEMBLY_LODS.get(at)) {
        setup.set_lod(*level);
    }
    materials(ui, texts, setup);
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::FileHeading));
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let choose = texts.get(T::ChooseFile);
            if button::button(ui, Kind::Ghost, Size::Normal, &choose, true).clicked() {
                action = Some(Action::ChooseFile);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                path_field(ui, texts.get(T::FileNone), setup.file());
            });
        });
    });
    refusal_notice(ui, texts, setup.refusal());
    let pressed = foot(
        ui,
        texts,
        (T::Start, setup.is_ready()),
        ExtractionText::Close,
    );
    action.or(match pressed {
        Some(true) => Some(Action::Submit),
        Some(false) => Some(Action::Close),
        None => None,
    })
}

fn materials(ui: &mut Ui, texts: &Texts, setup: &mut AssemblyExportSetup) {
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::MaterialsHeading));
    for materials in ASSEMBLY_MATERIALS {
        let label = texts.get(match materials {
            ModelMaterials::Colours => T::MaterialsColours,
            ModelMaterials::Textures => T::MaterialsTextures,
        });
        let enabled = ASSEMBLY_TEXTURES_AVAILABLE || materials == ModelMaterials::Colours;
        if toggle::radio(ui, setup.materials() == materials, &label, enabled).clicked() {
            setup.set_materials(materials);
        }
    }
    if !ASSEMBLY_TEXTURES_AVAILABLE {
        ui.label(
            plain(texts.get(T::TexturesUnavailable), FS_XS, Weight::Regular)
                .color(palette::GRAY_2_TEXT),
        );
    }
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::TextureMipHeading));
    let mips = setup.materials() == ModelMaterials::Textures;
    let labels = MODEL_TEXTURE_MIPS.map(|mip| {
        texts.format(
            T::TextureMip,
            &[("level", mip.into()), ("divisor", (1_u32 << mip).into())],
        )
    });
    let active = MODEL_TEXTURE_MIPS
        .iter()
        .position(|mip| *mip == setup.texture_mip());
    let chosen = ui
        .add_enabled_ui(mips, |ui| {
            ui.horizontal(|ui| tab::segments(ui, &labels, active)).inner
        })
        .inner;
    if let Some(mip) = chosen.and_then(|at| MODEL_TEXTURE_MIPS.get(at)) {
        setup.set_texture_mip(*mip);
    }
    ui.add_space(MODAL_GROUP_GAP);
    let mut colours = setup.vertex_colours();
    if toggle::checkbox(ui, &mut colours, &texts.get(T::VertexColours)).changed() {
        setup.set_vertex_colours(colours);
    }
}
