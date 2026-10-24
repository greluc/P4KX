// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the 3D view of the model preview, taken from the `modelPreview` entry of
//! `docs/design/p4kx-desktop-spec.json`.

use super::tokens::SPACE_2;

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
