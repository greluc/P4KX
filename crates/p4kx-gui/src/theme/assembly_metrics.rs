// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The metrics of the 3D view of the Entities tab, taken from the `assemblyView` entry of
//! `docs/design/p4kx-desktop-spec.json`.

use super::desktop::ROW_HEIGHT;
use super::model_metrics::{MODEL_BAR_HEIGHT, MODEL_OVERLAY_GAP, MODEL_VIEW_MIN_HEIGHT};
use super::sound_metrics::{FACTS_HEIGHT, PANEL_CHROME};

/// The height of the 3D view the panel opens with.
pub const ASSEMBLY_VIEW_HEIGHT: f32 = 280.0;
/// The rows of the Loadout tree that stay visible however the panel is resized.
pub const ASSEMBLY_TREE_MIN_ROWS: f32 = 3.0;
/// The rows of the bar under the 3D view: the choices, and the status.
pub const ASSEMBLY_BAR_ROWS: f32 = 2.0;
/// The height of the view panel as it opens: the facts bar, the 3D view, the gap and the two
/// rows of the bar, and what the panel adds around them.
pub const ASSEMBLY_PANEL_HEIGHT: f32 = FACTS_HEIGHT
    + ASSEMBLY_VIEW_HEIGHT
    + MODEL_OVERLAY_GAP
    + ASSEMBLY_BAR_ROWS * MODEL_BAR_HEIGHT
    + PANEL_CHROME;
/// The share of the room right of the picker the view panel opens with when that is more than
/// [`ASSEMBLY_PANEL_HEIGHT`].
pub const ASSEMBLY_PANEL_SHARE: f32 = 0.55;
/// The least height of the view panel: the same with the smallest 3D view.
pub const ASSEMBLY_PANEL_MIN_HEIGHT: f32 = FACTS_HEIGHT
    + MODEL_VIEW_MIN_HEIGHT
    + MODEL_OVERLAY_GAP
    + ASSEMBLY_BAR_ROWS * MODEL_BAR_HEIGHT
    + PANEL_CHROME;
/// The least height the Loadout tree keeps: its head row and [`ASSEMBLY_TREE_MIN_ROWS`] ports.
pub const ASSEMBLY_TREE_MIN_HEIGHT: f32 = ROW_HEIGHT * (1.0 + ASSEMBLY_TREE_MIN_ROWS);
