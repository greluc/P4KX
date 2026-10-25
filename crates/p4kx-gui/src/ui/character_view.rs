// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{Color32, Label, Ui};

use super::model_playback::{self, ViewerAsk};
use super::model_preview::problem_text;
use super::model_view;
use crate::theme::desktop::SPINNER_INLINE;
use crate::theme::model_metrics::MODEL_BAR_HEIGHT;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::widgets::text::plain;
use crate::{
    CharacterText as T, CharactersTab, EntityText, Model, Scene, Texts, ViewFailure, ViewStage,
    ViewerSupport, format,
};

enum Ask {
    Stop,
    Restart,
    View(ViewerAsk),
}

/// Shows the 3D view of the Character: its facts, the status of the build, the view with the
/// playback bar, or why there is no view yet.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let support = model.viewer_support().clone();
    let ask = {
        let (tab, texts) = model.characters_with_texts();
        ui.set_min_height(ui.available_height());
        match tab.scene().cloned() {
            Some(scene) => ready(ui, texts, tab, &scene, &support),
            None => waiting(ui, texts, tab),
        }
    };
    let tab = model.characters_mut();
    match ask {
        Some(Ask::Stop) => tab.cancel(),
        Some(Ask::Restart) => tab.restart(),
        Some(Ask::View(view)) => {
            if let (Some(camera), Some(current)) = (view.camera, tab.camera_mut()) {
                *current = camera;
            }
            if let (Some(control), Some(playback)) = (view.control, tab.playback_mut()) {
                playback.apply(control);
            }
        }
        None => {}
    }
}

fn ready(
    ui: &mut Ui,
    texts: &Texts,
    tab: &CharactersTab,
    scene: &Arc<Scene>,
    support: &ViewerSupport,
) -> Option<Ask> {
    let problem = support.problem(scene.serial());
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        model_view::facts(ui, texts, scene);
        frames::preview_body()
            .show(ui, |ui| {
                let stopped = status(ui, texts, tab);
                if let Some(problem) = problem {
                    let (colour, text) = problem_text(texts, &tab.name(), &problem);
                    ui.colored_label(colour, text);
                    return stopped;
                }
                let camera = tab.camera()?;
                let name = tab.name();
                let asked = model_playback::view(ui, texts, &name, (scene, camera), tab.playback());
                stopped.or(Some(Ask::View(asked)))
            })
            .inner
    })
    .inner
}

fn waiting(ui: &mut Ui, texts: &Texts, tab: &CharactersTab) -> Option<Ask> {
    frames::preview_body()
        .show(ui, |ui| status(ui, texts, tab))
        .inner
}

fn line(ui: &mut Ui, text: String, colour: Color32) {
    ui.add(Label::new(plain(text, FS_SM, Weight::Regular).color(colour)).truncate());
}

fn status(ui: &mut Ui, texts: &Texts, tab: &CharactersTab) -> Option<Ask> {
    if tab.progress().is_none() && !tab.is_cancelled() && tab.failure().is_none() {
        return None;
    }
    let mut ask = None;
    ui.horizontal(|ui| {
        ui.set_min_height(MODEL_BAR_HEIGHT);
        if let Some(progress) = tab.progress() {
            spinner(ui, SPINNER_INLINE);
            let text = match progress.stage {
                ViewStage::Reading => texts.format(
                    EntityText::ViewReading,
                    &[(
                        "count",
                        format::number(progress.done, texts.language()).into(),
                    )],
                ),
                _ => texts.get(T::ViewPreparing),
            };
            ui.add(
                Label::new(plain(text, FS_XS, Weight::Regular).color(palette::GRAY_1)).truncate(),
            );
            let stop = texts.get(EntityText::ViewStop);
            if button::button(ui, Kind::Outline, Size::Compact, &stop, true).clicked() {
                ask = Some(Ask::Stop);
            }
        } else if tab.is_cancelled() {
            line(ui, texts.get(EntityText::ViewStopped), palette::GRAY_1);
            let again = texts.get(EntityText::ViewRestart);
            if button::button(ui, Kind::Outline, Size::Compact, &again, true).clicked() {
                ask = Some(Ask::Restart);
            }
        } else if let Some((colour, text)) =
            tab.failure().map(|failure| failure_text(texts, failure))
        {
            line(ui, text, colour);
        }
    });
    ask
}

fn failure_text(texts: &Texts, failure: &ViewFailure) -> (Color32, String) {
    match failure {
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
    }
}
