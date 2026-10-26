// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the 3D view of the model preview, taken from the `modelPreview` entry of
//! `docs/design/p4kx-desktop-spec.json`.

use super::desktop::{PREVIEW_PATH_HEIGHT, ROW_HEIGHT};
use super::sound_metrics::{FACTS_HEIGHT, PANEL_CHROME};
use super::tokens::{SPACE_2, TOUCH_TARGET};

/// The smallest side of the 3D view of the model preview.
pub const MODEL_VIEW_MIN_HEIGHT: f32 = 96.0;
/// The gap between the edge of the 3D view and the controls laid over it.
pub const MODEL_OVERLAY_GAP: f32 = SPACE_2;
/// The least width the facts bar leaves for the overview of the controls before it is left out.
pub const MODEL_HINT_MIN_WIDTH: f32 = 120.0;
/// The width of the list of animations beside the 3D view.
pub const MODEL_ANIMATIONS_WIDTH: f32 = 240.0;
/// The width in points of the lines that draw the skeleton over the model.
pub const MODEL_BONE_STROKE: f32 = 2.0;
/// The height of the bar with the playback controls under the 3D view: a row, as high as the cell
/// of a checkbox.
pub const MODEL_BAR_HEIGHT: f32 = ROW_HEIGHT;
/// The height of the 3D view the panel of a model opens with, well above [`MODEL_VIEW_MIN_HEIGHT`].
pub const MODEL_VIEW_HEIGHT: f32 = 160.0;
/// The default height of the preview while it shows a model: the tab bar, the path line, the
/// facts bar, the 3D view, the gap and the playback bar, and what the panel adds around them.
pub const PREVIEW_MODEL_HEIGHT: f32 = TOUCH_TARGET
    + PREVIEW_PATH_HEIGHT
    + FACTS_HEIGHT
    + MODEL_VIEW_HEIGHT
    + MODEL_OVERLAY_GAP
    + MODEL_BAR_HEIGHT
    + PANEL_CHROME;
/// The least height of the preview while it shows a model: the default with the 3D view at
/// [`MODEL_VIEW_MIN_HEIGHT`], so the playback bar never overlaps the view.
pub const PREVIEW_MODEL_MIN_HEIGHT: f32 =
    PREVIEW_MODEL_HEIGHT - MODEL_VIEW_HEIGHT + MODEL_VIEW_MIN_HEIGHT;
