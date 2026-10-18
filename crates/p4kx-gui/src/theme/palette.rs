//! The colour tokens of the KRT design system, named as in its `tokens.json`.

use egui::Color32;

/// `color-primary`, the house orange: the filled CTA, headings, links, focus and HUD brackets.
pub const PRIMARY: Color32 = Color32::from_rgb(0xE7, 0x7E, 0x23);
/// `color-white`: data readouts, selected-control text, text on danger and success fills.
pub const WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// `color-bg-black`: the canvas and the text on the orange CTA.
pub const BLACK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
/// `color-accent-light`: the hover state of every orange element.
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xEE, 0xB6, 0x4B);
/// `color-accent-dark`: elevated chrome and deep accents, never text on black.
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xC4, 0x5C, 0x00);
/// `color-gray-1`: body text, labels and table text.
pub const GRAY_1: Color32 = Color32::from_rgb(0xD2, 0xD2, 0xD2);
/// `color-gray-2`: control borders, the scrollbar thumb and disabled fills, never text.
pub const GRAY_2: Color32 = Color32::from_rgb(0x64, 0x64, 0x64);
/// `color-gray-3`: decorative hairlines, the row hover and the count-badge fill.
pub const GRAY_3: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
/// `color-gray-4`: the standard dark surface of header, footer, tables and sidebar.
pub const GRAY_4: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
/// `color-bg-dark-gray`, alias of [`GRAY_4`].
pub const BG_DARK_GRAY: Color32 = GRAY_4;
/// `color-surface-input`: the fill of inputs, table heads and controls.
pub const SURFACE_INPUT: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x1C);
/// `color-danger`: danger as a fill or border.
pub const DANGER: Color32 = Color32::from_rgb(0xA3, 0x00, 0x0A);
/// `color-success`: success as a fill or border.
pub const SUCCESS: Color32 = Color32::from_rgb(0x23, 0x9E, 0x33);
/// `color-warning`: warning as a border and as text on black.
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x3F);
/// `color-info`: info as a fill or border.
pub const INFO: Color32 = Color32::from_rgb(0x35, 0x5D, 0xDC);
/// `color-secondary`: the cross-squadron highlight.
pub const SECONDARY: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x3F);
/// `color-danger-text`: danger as text.
pub const DANGER_TEXT: Color32 = Color32::from_rgb(0xF2, 0x56, 0x4B);
/// `color-info-text`: info as text.
pub const INFO_TEXT: Color32 = Color32::from_rgb(0x6C, 0x93, 0xEF);
/// `color-success-text`: success as text.
pub const SUCCESS_TEXT: Color32 = Color32::from_rgb(0x2E, 0xBC, 0x3D);
/// `color-gray-2-text`: muted text, hints, placeholders and inactive tabs.
pub const GRAY_2_TEXT: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);
/// `color-danger-hover`: the hover fill of the danger button.
pub const DANGER_HOVER: Color32 = Color32::from_rgb(0xD4, 0x1A, 0x25);
/// `color-success-hover`: the hover fill of the success button.
pub const SUCCESS_HOVER: Color32 = Color32::from_rgb(0x2E, 0xBC, 0x3D);
/// `action-primary`: the one filled CTA per context.
pub const ACTION_PRIMARY: Color32 = PRIMARY;
/// `action-emphasis`: the outline secondary action.
pub const ACTION_EMPHASIS: Color32 = PRIMARY;
/// `action-neutral`: the ghost border.
pub const ACTION_NEUTRAL: Color32 = GRAY_2;
/// `data-fg`: bright neutral data readouts.
pub const DATA_FG: Color32 = WHITE;
/// `surface-canvas`, step 0 of the surface ladder.
pub const SURFACE_CANVAS: Color32 = BLACK;
/// `surface-panel`, step 1: the work surface of toolbar, table and footer.
pub const SURFACE_PANEL: Color32 = GRAY_4;
/// `surface-inset`, step 2: table head, inputs and control fill.
pub const SURFACE_INSET: Color32 = SURFACE_INPUT;
/// `surface-raised`, step 3: row hover and control hover.
pub const SURFACE_RAISED: Color32 = GRAY_3;
/// `control-bg`: the resting fill of every non-CTA control.
pub const CONTROL_BG: Color32 = SURFACE_INPUT;
/// `control-bg-hover`: the hover fill of controls.
pub const CONTROL_BG_HOVER: Color32 = GRAY_3;
/// `control-border`: the border of every non-CTA control.
pub const CONTROL_BORDER: Color32 = GRAY_2;
/// `control-fg`: control label text.
pub const CONTROL_FG: Color32 = GRAY_1;
/// `control-selected-bg`: the orange tint of a selected segment.
pub const CONTROL_SELECTED_BG: Color32 = Color32::from_rgba_unmultiplied_const(231, 126, 35, 46);
/// `control-selected-border`: the border and underline of a selected segment.
pub const CONTROL_SELECTED_BORDER: Color32 = PRIMARY;
/// `control-selected-fg`: the text of a selected segment.
pub const CONTROL_SELECTED_FG: Color32 = WHITE;
