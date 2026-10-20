//! The tints of table and list rows; they are re-exported by [`super::desktop`].

use egui::Color32;

/// The hover fill of a list row (`.master-row:hover`), `rgba(255,255,255,0.03)`.
pub const LIST_ROW_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 8);
/// The hover fill of a path-tree row (`.tree-row--mid:hover`), `rgba(255,255,255,0.025)`.
pub const TREE_ROW_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 6);
/// The zebra stripe of a table, `rgba(255,255,255,0.02)`.
pub const ZEBRA: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 5);
