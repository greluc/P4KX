use egui::{Align, Layout, Ui};

use super::extraction::{foot, group_label, path_field, refusal_notice};
use super::{Dialog, Requests};
use crate::theme::desktop::MODAL_GROUP_GAP;
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone};
use crate::widgets::text::plain;
use crate::widgets::toggle;
use crate::{
    ExtractionText, MODEL_MATERIALS, Model, ModelExportSetup, ModelExportText as T, Text, Texts,
    format,
};
use p4kx_settings::ModelMaterials;

enum Action {
    Submit,
    Close,
    ChooseFile,
}

/// Shows the Export as GLB setup while it is open and applies what the user chose.
pub(super) fn setup(ui: &Ui, model: &mut Model, requests: &mut Requests) {
    let Some((setup, texts)) = model.model_export_setup_with_texts() else {
        return;
    };
    let title = texts.get(T::SetupTitle);
    let close_dialog = texts.get(Text::CloseDialog);
    let shown =
        frames::modal(frames::modal_id("model-export-setup"), Tone::Normal).show(ui.ctx(), |ui| {
            ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
            let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
            let action = choices(ui, texts, setup);
            if closed { Some(Action::Close) } else { action }
        });
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
            requests.settings_changed |= model.submit_model_export(move || ctx.request_repaint());
        }
        Some(Action::Close) => model.close_model_export_setup(),
        Some(Action::ChooseFile) => requests.dialog = Some(Dialog::ModelFile),
        None if close => model.close_model_export_setup(),
        None => {}
    }
}

fn choices(ui: &mut Ui, texts: &Texts, setup: &mut ModelExportSetup) -> Option<Action> {
    let mut action = None;
    let name = texts.format(T::Model, &[("name", setup.name().to_owned().into())]);
    ui.label(plain(name, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT));
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::LodHeading));
    let language = texts.language();
    for lod in setup.lods().to_vec() {
        let triangles = u64::try_from(lod.triangles).unwrap_or(u64::MAX);
        let label = texts.format(
            T::Lod,
            &[
                ("level", lod.level.into()),
                ("triangles", triangles.into()),
                ("number", format::number(triangles, language).into()),
            ],
        );
        if toggle::radio(ui, setup.lod() == lod.level, &label, true).clicked() {
            setup.set_lod(lod.level);
        }
    }
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::MaterialsHeading));
    for materials in MODEL_MATERIALS {
        let label = texts.get(match materials {
            ModelMaterials::Colours => T::MaterialsColours,
        });
        if toggle::radio(ui, setup.materials() == materials, &label, true).clicked() {
            setup.set_materials(materials);
        }
    }
    let mut colours = setup.vertex_colours();
    if toggle::checkbox(ui, &mut colours, &texts.get(T::VertexColours)).changed() {
        setup.set_vertex_colours(colours);
    }
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
