// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{Color32, Ui};
use fluent_bundle::FluentValue;

use super::model_view;
use crate::theme::desktop::SPINNER_INLINE;
use crate::theme::palette;
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::{
    Camera, MAX_SCENE_TRIANGLES, MAX_SCENE_VERTICES, MODEL_FILE_LIMIT, ModelPreview, ModelStatus,
    ModelText, Problem, Scene, SceneError, Text, Texts, Unavailable, ViewerSupport, format,
};

/// Shows the Model tab of the model Entry at `path`: its facts and the 3D view, or the reason
/// there is none. Returns the camera when the user moved it.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    path: &str,
    (preview, support): (&ModelPreview, &ViewerSupport),
) -> Option<Camera> {
    match preview.status() {
        ModelStatus::Unshowable(error) => {
            let (colour, text) = unshowable(texts, path, error);
            note(ui, colour, text);
        }
        ModelStatus::Ready(scene) => {
            let camera = preview.camera()?;
            return ready(ui, texts, path, (scene, camera), support);
        }
        status => unread(ui, texts, path, status),
    }
    None
}

fn note(ui: &mut Ui, colour: Color32, text: String) {
    frames::preview_body().show(ui, |ui| ui.colored_label(colour, text));
}

/// Shows why the model of `path` is not there to show: it is being read, cannot be read, the
/// Archive changed or the file is too large. A model that was read shows nothing here.
pub(super) fn unread(ui: &mut Ui, texts: &Texts, path: &str, status: ModelStatus<'_>) {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    match status {
        ModelStatus::Loading => {
            frames::preview_body().show(ui, |ui| {
                ui.horizontal(|ui| {
                    spinner(ui, SPINNER_INLINE);
                    ui.label(texts.format(ModelText::Loading, &[path_arg()]));
                });
            });
        }
        ModelStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.format(Text::PreviewStopped, &[path_arg()]),
                |reason| {
                    let reason = ("reason", FluentValue::from(reason.to_owned()));
                    texts.format(ModelText::Failed, &[path_arg(), reason])
                },
            );
            note(ui, palette::DANGER_TEXT, text);
        }
        ModelStatus::ArchiveChanged => {
            note(ui, palette::WARNING, texts.get(Text::PreviewArchiveChanged));
        }
        ModelStatus::FileTooLarge(size) => {
            let language = texts.language();
            let args = [
                path_arg(),
                ("size", format::size(size, language).into()),
                ("limit", format::size(MODEL_FILE_LIMIT, language).into()),
            ];
            note(
                ui,
                palette::GRAY_1,
                texts.format(ModelText::FileTooLarge, &args),
            );
        }
        ModelStatus::Unshowable(_) | ModelStatus::Ready(_) => {}
    }
}

fn unshowable(texts: &Texts, path: &str, error: SceneError) -> (Color32, String) {
    let path_arg = ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    match error {
        SceneError::Empty => (palette::GRAY_1, texts.format(ModelText::Empty, &[path_arg])),
        SceneError::NotFinite => (
            palette::DANGER_TEXT,
            texts.format(ModelText::NotFinite, &[path_arg]),
        ),
        SceneError::TooLarge {
            vertices,
            triangles,
        } => {
            let count =
                |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), language);
            let args = [
                path_arg,
                ("vertices", count(vertices).into()),
                ("triangles", count(triangles).into()),
                ("max_vertices", count(MAX_SCENE_VERTICES).into()),
                ("max_triangles", count(MAX_SCENE_TRIANGLES).into()),
            ];
            (palette::GRAY_1, texts.format(ModelText::TooLarge, &args))
        }
    }
}

fn ready(
    ui: &mut Ui,
    texts: &Texts,
    path: &str,
    (scene, camera): (&Arc<Scene>, &Camera),
    support: &ViewerSupport,
) -> Option<Camera> {
    let problem = support.problem(scene.serial());
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        model_view::facts(ui, texts, scene);
        frames::preview_body()
            .show(ui, |ui| match problem {
                Some(problem) => {
                    let (colour, text) = problem_text(texts, path, &problem);
                    ui.colored_label(colour, text);
                    None
                }
                None => model_view::viewport(ui, texts, path, (scene, camera)),
            })
            .inner
    })
    .inner
}

fn sentence(reason: &str) -> String {
    let reason = reason.trim_end();
    if reason.ends_with(['.', '!', '?']) {
        reason.to_owned()
    } else {
        format!("{reason}.")
    }
}

fn problem_text(texts: &Texts, path: &str, problem: &Problem) -> (Color32, String) {
    let language = texts.language();
    let limit = |value: u64| ("limit", format::number(value, language).into());
    let text = |text: ModelText, args: &[(&str, FluentValue<'_>)]| texts.format(text, args);
    match problem {
        Problem::Unavailable(Unavailable::NoDevice) => {
            (palette::DANGER_TEXT, text(ModelText::GpuNone, &[]))
        }
        Problem::Unavailable(Unavailable::TextureSide(side)) => (
            palette::DANGER_TEXT,
            text(ModelText::GpuTextureSide, &[limit(u64::from(*side))]),
        ),
        Problem::Unavailable(Unavailable::BufferSize(bytes)) => (
            palette::DANGER_TEXT,
            text(ModelText::GpuBufferSize, &[limit(*bytes)]),
        ),
        Problem::Unavailable(Unavailable::Setup(reason)) => (
            palette::DANGER_TEXT,
            text(ModelText::GpuSetup, &[("reason", sentence(reason).into())]),
        ),
        Problem::Lost => (palette::DANGER_TEXT, text(ModelText::GpuLost, &[])),
        Problem::Failed => (palette::DANGER_TEXT, text(ModelText::GpuFailed, &[])),
        Problem::Rejected => (
            palette::GRAY_1,
            text(ModelText::GpuRejected, &[("path", path.to_owned().into())]),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::sentence;

    #[test]
    fn a_reason_without_a_final_full_stop_gets_one() {
        assert_eq!(sentence("shader refused"), "shader refused.", "added");
        assert_eq!(sentence("shader refused. "), "shader refused.", "kept");
        assert_eq!(sentence("really?"), "really?", "other end marks stay");
    }
}
