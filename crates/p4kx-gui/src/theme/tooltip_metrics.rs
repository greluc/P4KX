//! The metrics of the tooltip, taken from the `tooltip` entry of
//! `docs/design/p4kx-desktop-spec.json`; they are re-exported by [`super::desktop`].

use egui::Color32;

use super::tokens::{REM, SPACE_2};

/// The widest a tooltip gets.
pub const TOOLTIP_MAX_WIDTH: f32 = 272.0;
/// The horizontal padding of a tooltip, 0.6 rem.
pub const TOOLTIP_PADDING_X: f32 = 0.6 * REM;
/// The vertical padding of a tooltip, `space-2`.
pub const TOOLTIP_PADDING_Y: f32 = SPACE_2;
/// The font size of a tooltip, 0.78 rem.
pub const TOOLTIP_SIZE: f32 = 0.78 * REM;
/// The line height of a tooltip as a multiple of its font size.
pub const TOOLTIP_LINE_HEIGHT: f32 = 1.4;
/// The fill of a tooltip, `rgba(0,0,0,0.95)`.
pub const TOOLTIP_FILL: Color32 = Color32::from_rgba_unmultiplied_const(0, 0, 0, 242);
