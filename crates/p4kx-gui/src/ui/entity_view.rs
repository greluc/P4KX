// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{Align, Color32, Label, Layout, Ui, UiBuilder};
use fluent_bundle::FluentValue;
use p4kx_settings::ModelMaterials;

use super::model_preview::problem_text;
use super::model_view::canvas;
use super::search::eyebrow;
use crate::theme::assembly_metrics::ASSEMBLY_TREE_MIN_HEIGHT;
use crate::theme::desktop::{AUDIO_CONTROL_GAP, SPINNER_INLINE};
use crate::theme::model_metrics::{MODEL_BAR_HEIGHT, MODEL_OVERLAY_GAP};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, Weight};
use crate::viewer::{AssemblyDraw, Look, Subject};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::widgets::summary::{Fact, FactsStrip};
use crate::widgets::tab;
use crate::widgets::text::plain;
use crate::{
    ASSEMBLY_LODS, AssemblyScene, AssemblyView, Camera, EntitiesTab, EntityText, Language,
    ModelText, Texts, ViewFailure, ViewProgress, ViewStage, ViewerSupport, format,
};

const MATERIALS: [ModelMaterials; 2] = [ModelMaterials::Colours, ModelMaterials::Textures];

enum Ask {
    Lod(u32),
    Materials(ModelMaterials),
    ShowAll,
    Stop,
    Restart,
    Camera(Camera),
}

/// The height the Loadout tree keeps below the view panel.
pub(super) const TREE_MIN_HEIGHT: f32 = ASSEMBLY_TREE_MIN_HEIGHT;

/// Shows the 3D view of the Loadout of the selected Entity: the facts of the Assembly, the view
/// and under it the choices of detail and materials, or why there is no view yet.
pub(super) fn show(ui: &mut Ui, tab: &mut EntitiesTab, texts: &Texts, support: &ViewerSupport) {
    ui.set_min_height(ui.available_height());
    let Some(session) = tab.loadout_mut() else {
        return;
    };
    let name = session.name().to_owned();
    let view = session.assembly_mut();
    let ask = if let Some(scene) = view.scene().cloned() {
        ready(ui, texts, &name, (view, &scene), support)
    } else {
        waiting(ui, texts, view)
    };
    match ask {
        Some(Ask::Lod(level)) => view.set_lod(level),
        Some(Ask::Materials(materials)) => view.set_materials(materials),
        Some(Ask::ShowAll) => view.show_all(),
        Some(Ask::Stop) => view.cancel(),
        Some(Ask::Restart) => view.restart(),
        Some(Ask::Camera(camera)) => {
            if let Some(current) = view.camera_mut() {
                *current = camera;
            }
        }
        None => {}
    }
}

fn ready(
    ui: &mut Ui,
    texts: &Texts,
    name: &str,
    (view, scene): (&mut AssemblyView, &Arc<AssemblyScene>),
    support: &ViewerSupport,
) -> Option<Ask> {
    let problem = support.problem(scene.serial());
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        facts(ui, texts, scene);
        frames::preview_body()
            .show(ui, |ui| {
                if let Some(problem) = problem {
                    let (colour, text) = problem_text(texts, name, &problem);
                    ui.colored_label(colour, text);
                    return None;
                }
                viewport(ui, texts, name, (view, scene))
            })
            .inner
    })
    .inner
}

fn viewport(
    ui: &mut Ui,
    texts: &Texts,
    name: &str,
    (view, scene): (&mut AssemblyView, &Arc<AssemblyScene>),
) -> Option<Ask> {
    let camera = view.camera().cloned()?;
    let reserve = MODEL_OVERLAY_GAP + MODEL_BAR_HEIGHT;
    let below = ui
        .max_rect()
        .with_min_y(ui.max_rect().bottom() - MODEL_BAR_HEIGHT);
    let describe = || {
        let args = [
            ("name", FluentValue::from(name.to_owned())),
            ("parts", scene.drawn_parts().to_string().into()),
            ("triangles", scene.placed_triangles().to_string().into()),
        ];
        texts.format(EntityText::ViewLabel, &args)
    };
    let pick = |uniforms: &crate::Uniforms, pixels: u32| -> Option<Subject> {
        let height = f32::from(u16::try_from(pixels).unwrap_or(u16::MAX));
        let AssemblyDraw { scene, list } = view.draw(&uniforms.view_proj, height)?;
        Some(Subject::Assembly(AssemblyDraw { scene, list }))
    };
    let moved = canvas(
        ui,
        (texts, &describe),
        (pick, &camera),
        (Look::default(), reserve),
    );
    let chosen = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(below)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| bar(ui, texts, view),
        )
        .inner;
    chosen.or_else(|| moved.map(Ask::Camera))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FactsKey {
    serial: u64,
    language: Language,
    pixels_per_point: f32,
}

fn facts(ui: &mut Ui, texts: &Texts, scene: &AssemblyScene) {
    let id = ui.scope_id().with("p4kx-assembly-facts");
    let key = FactsKey {
        serial: scene.serial(),
        language: texts.language(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    let cached = ui.data(|data| data.get_temp::<(FactsKey, Arc<FactsStrip>)>(id));
    let strip = match cached {
        Some((cached, strip)) if cached == key => strip,
        _ => {
            let strip = Arc::new(lay_out(ui, texts, scene));
            ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&strip))));
            strip
        }
    };
    strip.show(ui);
}

fn lay_out(ui: &Ui, texts: &Texts, scene: &AssemblyScene) -> FactsStrip {
    let language = texts.language();
    let count = |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), language);
    let bounds = scene.bounds();
    let side = |axis: usize| {
        let min = bounds.min.get(axis).copied().unwrap_or(0.0);
        let max = bounds.max.get(axis).copied().unwrap_or(0.0);
        format::metres(max - min, language)
    };
    let extent = texts.format(
        ModelText::Dimensions,
        &[
            ("width", side(0).into()),
            ("depth", side(1).into()),
            ("height", side(2).into()),
        ],
    );
    let keys = [
        texts.get(EntityText::ViewParts),
        texts.get(EntityText::ViewMeshes),
        texts.get(EntityText::ViewTriangles),
        texts.get(ModelText::Extent),
    ];
    let values = [
        count(scene.drawn_parts()),
        count(scene.meshes()),
        count(scene.placed_triangles()),
        extent,
    ];
    let laid: Vec<Fact<'_>> = keys
        .iter()
        .zip(&values)
        .map(|(key, value)| Fact { key, value })
        .collect();
    FactsStrip::lay_out(ui, &laid)
}

fn bar(ui: &mut Ui, texts: &Texts, view: &AssemblyView) -> Option<Ask> {
    let mut ask = None;
    ui.spacing_mut().item_spacing.x = AUDIO_CONTROL_GAP;
    ui.label(eyebrow(&texts.get(EntityText::ViewLod)));
    let levels = ASSEMBLY_LODS.map(|level| level.to_string());
    let active = ASSEMBLY_LODS.iter().position(|level| *level == view.lod());
    if let Some(at) = tab::segments(ui, &levels, active)
        && let Some(level) = ASSEMBLY_LODS.get(at)
    {
        ask = Some(Ask::Lod(*level));
    }
    ui.add_space(AUDIO_CONTROL_GAP);
    ui.label(eyebrow(&texts.get(EntityText::ViewMaterials)));
    let names = MATERIALS.map(|materials| {
        texts.get(match materials {
            ModelMaterials::Colours => EntityText::ViewColours,
            ModelMaterials::Textures => EntityText::ViewTextures,
        })
    });
    let active = MATERIALS
        .iter()
        .position(|materials| *materials == view.materials());
    if let Some(at) = tab::segments(ui, &names, active)
        && let Some(materials) = MATERIALS.get(at)
    {
        ask = Some(Ask::Materials(*materials));
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ask = status(ui, texts, view).or_else(|| ask.take());
        if view.hidden_count() > 0 {
            let label = texts.format(
                EntityText::ViewShowAll,
                &[("count", view.hidden_count().to_string().into())],
            );
            if button::button(ui, Kind::Ghost, Size::Compact, &label, true).clicked() {
                ask = Some(Ask::ShowAll);
            }
        }
    });
    ask
}

fn status(ui: &mut Ui, texts: &Texts, view: &AssemblyView) -> Option<Ask> {
    let mut ask = None;
    let line = |ui: &mut Ui, text: String, colour: Color32| {
        ui.add(Label::new(plain(text, FS_XS, Weight::Regular).color(colour)).truncate());
    };
    if let Some(progress) = view.progress() {
        let stop = texts.get(EntityText::ViewStop);
        if button::button(ui, Kind::Outline, Size::Compact, &stop, true).clicked() {
            ask = Some(Ask::Stop);
        }
        note(ui, texts, progress);
        spinner(ui, SPINNER_INLINE);
    } else if view.is_cancelled() {
        let again = texts.get(EntityText::ViewRestart);
        if button::button(ui, Kind::Outline, Size::Compact, &again, true).clicked() {
            ask = Some(Ask::Restart);
        }
        line(ui, texts.get(EntityText::ViewStopped), palette::GRAY_1);
    } else if let Some((colour, text)) = failure_text(texts, view) {
        line(ui, text, colour);
    } else if view.issues() > 0 {
        let count = [("count", view.issues().to_string().into())];
        line(
            ui,
            texts.format(EntityText::ViewIssues, &count),
            palette::WARNING,
        );
    }
    ask
}

fn note(ui: &mut Ui, texts: &Texts, progress: ViewProgress) {
    let language = texts.language();
    let number = |value: u64| format::number(value, language);
    let text = match progress.stage {
        ViewStage::Reading => texts.format(
            EntityText::ViewReading,
            &[("count", number(progress.done).into())],
        ),
        ViewStage::Flattening => texts.format(
            EntityText::ViewFlattening,
            &[
                ("done", number(progress.done).into()),
                ("total", number(progress.total).into()),
            ],
        ),
        ViewStage::Textures => texts.format(
            EntityText::ViewTextureProgress,
            &[
                ("done", number(progress.done).into()),
                ("total", number(progress.total).into()),
            ],
        ),
    };
    ui.add(Label::new(plain(text, FS_XS, Weight::Regular).color(palette::GRAY_1)).truncate());
}

fn failure_text(texts: &Texts, view: &AssemblyView) -> Option<(Color32, String)> {
    let failure = view.failure()?;
    Some(match failure {
        ViewFailure::Empty => (palette::GRAY_1, texts.get(EntityText::ViewEmpty)),
        ViewFailure::ArchiveChanged => {
            (palette::WARNING, texts.get(EntityText::ViewArchiveChanged))
        }
        ViewFailure::Key => (palette::DANGER_TEXT, texts.get(EntityText::ViewKey)),
        ViewFailure::Failed(reason) => (
            palette::DANGER_TEXT,
            texts.format(EntityText::ViewFailed, &[("reason", reason.clone().into())]),
        ),
        ViewFailure::Stopped => (palette::GRAY_1, texts.get(EntityText::ViewStopped)),
    })
}

fn waiting(ui: &mut Ui, texts: &Texts, view: &AssemblyView) -> Option<Ask> {
    let mut ask = None;
    frames::preview_body().show(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(progress) = view.progress() {
                spinner(ui, SPINNER_INLINE);
                note(ui, texts, progress);
                let stop = texts.get(EntityText::ViewStop);
                if button::button(ui, Kind::Outline, Size::Compact, &stop, true).clicked() {
                    ask = Some(Ask::Stop);
                }
            } else if let Some((colour, text)) = failure_text(texts, view) {
                ui.add(Label::new(
                    plain(text, FS_SM, Weight::Regular).color(colour),
                ));
            } else if view.is_cancelled() {
                let text = texts.get(EntityText::ViewStopped);
                ui.add(Label::new(
                    plain(text, FS_SM, Weight::Regular).color(palette::GRAY_1),
                ));
                let again = texts.get(EntityText::ViewRestart);
                if button::button(ui, Kind::Outline, Size::Compact, &again, true).clicked() {
                    ask = Some(Ask::Restart);
                }
            }
        });
    });
    ask
}
