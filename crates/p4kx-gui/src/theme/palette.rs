//! The colour tokens of the P4KX design, named as in the token list of the P4KX design.

use egui::Color32;

/// `color-primary`, the house orange: the filled CTA, headings, links, focus and HUD brackets.
pub const PRIMARY: Color32 = Color32::from_rgb(0xE7, 0x7E, 0x23);
/// `color-white`: data readouts, selected-control text, text on danger and success fills.
pub const WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// `color-bg-black`: the text on the orange CTA and on light fills, never a background.
pub const BLACK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
/// `color-canvas`: the page background and the boxes set into a panel.
pub const CANVAS: Color32 = Color32::from_rgb(0x0F, 0x0F, 0x0F);
/// `color-accent-light`: the hover state of every orange element.
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xEE, 0xB6, 0x4B);
/// `color-accent-dark`: admin chrome and deep accents, never text.
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xC4, 0x5C, 0x00);
/// `color-gray-1`: body text, labels and table text.
pub const GRAY_1: Color32 = Color32::from_rgb(0xD1, 0xD2, 0xD3);
/// `color-gray-2`: the border of every control, the scrollbar thumb and status dots, never text.
pub const GRAY_2: Color32 = Color32::from_rgb(0x8F, 0x93, 0x99);
/// `color-gray-3`: decorative hairlines and panel frames, never a control border.
pub const GRAY_3: Color32 = Color32::from_rgb(0x36, 0x37, 0x3B);
/// `color-gray-4`: the panel, the standard surface of tables, modals and the sidebar.
pub const GRAY_4: Color32 = Color32::from_rgb(0x28, 0x2C, 0x30);
/// `color-bg-dark-gray`, alias of [`GRAY_4`].
pub const BG_DARK_GRAY: Color32 = GRAY_4;
/// `color-surface-input`: the bar of header, footer, tab bar and table head, and inset fields.
pub const SURFACE_INPUT: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x21);
/// `color-control`: the resting fill of every non-CTA control and the table row hover.
pub const CONTROL: Color32 = Color32::from_rgb(0x37, 0x3E, 0x43);
/// `color-control-hover`: the hover fill of controls and segments.
pub const CONTROL_HOVER: Color32 = Color32::from_rgb(0x43, 0x4B, 0x51);
/// `color-count-badge`: the fill of a title counter.
pub const COUNT_BADGE: Color32 = Color32::from_rgb(0x2F, 0x39, 0x4C);
/// `color-danger`: danger as a fill or border.
pub const DANGER: Color32 = Color32::from_rgb(0xA3, 0x00, 0x0A);
/// `color-success`: success as a fill or border.
pub const SUCCESS: Color32 = Color32::from_rgb(0x23, 0x9E, 0x33);
/// `color-warning`: warning as a border and as text.
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x3F);
/// `color-info`: info as a fill or border.
pub const INFO: Color32 = Color32::from_rgb(0x35, 0x5D, 0xDC);
/// `color-secondary`: the cross-squadron highlight.
pub const SECONDARY: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x3F);
/// `color-danger-text`: danger as text.
pub const DANGER_TEXT: Color32 = Color32::from_rgb(0xFF, 0x7A, 0x70);
/// `color-info-text`: info as text.
pub const INFO_TEXT: Color32 = Color32::from_rgb(0x8F, 0xB2, 0xF5);
/// `color-success-text`: success as text.
pub const SUCCESS_TEXT: Color32 = Color32::from_rgb(0x4C, 0xCB, 0x5C);
/// `color-gray-2-text`: muted text, hints, placeholders and inactive tabs.
pub const GRAY_2_TEXT: Color32 = Color32::from_rgb(0xA2, 0xA5, 0xA9);
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
/// `surface-canvas`, step 0 of the surface ladder: the page background.
pub const SURFACE_CANVAS: Color32 = CANVAS;
/// `surface-panel`, step 1: the work surface of toolbar, table and footer.
pub const SURFACE_PANEL: Color32 = GRAY_4;
/// `surface-inset`: a box set into a panel.
pub const SURFACE_INSET: Color32 = CANVAS;
/// `surface-bar`: header, footer, tab bar and table head.
pub const SURFACE_BAR: Color32 = SURFACE_INPUT;
/// `surface-raised`: the `.btn-secondary` fill.
pub const SURFACE_RAISED: Color32 = GRAY_3;
/// `control-bg`: the resting fill of every non-CTA control.
pub const CONTROL_BG: Color32 = CONTROL;
/// `control-bg-hover`: the hover fill of controls.
pub const CONTROL_BG_HOVER: Color32 = CONTROL_HOVER;
/// `control-border`: the border of every non-CTA control.
pub const CONTROL_BORDER: Color32 = GRAY_2;
/// `control-fg`: control label text.
pub const CONTROL_FG: Color32 = GRAY_1;
/// `control-selected-bg`: the orange tint of a selected segment.
pub const CONTROL_SELECTED_BG: Color32 = Color32::from_rgba_unmultiplied_const(231, 126, 35, 66);
/// `control-selected-border`: the border and underline of a selected segment.
pub const CONTROL_SELECTED_BORDER: Color32 = PRIMARY;
/// `control-selected-fg`: the text of a selected segment.
pub const CONTROL_SELECTED_FG: Color32 = WHITE;
