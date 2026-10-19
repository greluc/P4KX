//! The tints of table and list rows; they are re-exported by [`super::desktop`].

use egui::Color32;

/// The hover fill of a tree or list row, `rgba(255,255,255,0.03)`.
pub const LIST_ROW_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 8);
/// The zebra stripe of a table, `rgba(255,255,255,0.02)`.
pub const ZEBRA: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 5);
