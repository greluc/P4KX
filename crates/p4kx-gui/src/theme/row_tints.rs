// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The tints of table and list rows; they are re-exported by [`super::desktop`].

use egui::Color32;

/// The hover fill of a list row (`.master-row:hover`), `rgba(255,255,255,0.03)`.
pub const LIST_ROW_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 8);
/// The hover fill of a path-tree row (`.tree-row--mid:hover`), `rgba(255,255,255,0.025)`.
pub const TREE_ROW_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 6);
/// The zebra stripe of a table, `table-stripe-bg`: `color-control` at 50 %, `#30353A` on the panel.
pub const ZEBRA: Color32 = super::palette::TABLE_STRIPE_BG;
