//! The colour tokens of the KRT design system (`colors_and_type.css`).

use egui::Color32;

/// Hausfarbe, the house orange: actions, selection and identity.
pub const PRIMARY: Color32 = Color32::from_rgb(0xE7, 0x7E, 0x23);
/// Zierfarbe hell, the hover state of the house orange.
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xEE, 0xB6, 0x4B);
/// Zierfarbe dunkel, for pressed and elevated accents.
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xC4, 0x5C, 0x00);
/// Weiß, for strong text.
pub const WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Schwarz, the canvas.
pub const BLACK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
/// Grauabstufung 1, body text on dark.
pub const GRAY_1: Color32 = Color32::from_rgb(0xD2, 0xD2, 0xD2);
/// Grauabstufung 2, for hairlines and disabled fills, never for text.
pub const GRAY_2: Color32 = Color32::from_rgb(0x64, 0x64, 0x64);
/// The accessible muted-text tint of Grauabstufung 2.
pub const GRAY_2_TEXT: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);
/// Grauabstufung 3, hairline borders and the hovered row.
pub const GRAY_3: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
/// Grauabstufung 4, the standard surface.
pub const GRAY_4: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
/// The fill of inputs and table heads, a half-step above Grauabstufung 4.
pub const SURFACE_INPUT: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x1C);
/// Danger as a fill or border.
pub const DANGER: Color32 = Color32::from_rgb(0xA3, 0x00, 0x0A);
/// Danger as text.
pub const DANGER_TEXT: Color32 = Color32::from_rgb(0xF2, 0x56, 0x4B);
/// Success as a fill or border.
pub const SUCCESS: Color32 = Color32::from_rgb(0x23, 0x9E, 0x33);
/// Warning, as fill and as text.
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x3F);
/// Info as a fill or border.
pub const INFO: Color32 = Color32::from_rgb(0x35, 0x5D, 0xDC);
