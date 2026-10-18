//! The KRT look: the design system's dark palette, square corners and Lato.

pub mod palette;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use egui::epaint::Shadow;
use egui::style::{Selection, TextCursorStyle, WidgetVisuals, Widgets};
use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Style,
    TextStyle, Theme, Visuals,
};

/// The font family name for Lato Bold, used by headings.
pub const BOLD_FAMILY: &str = "lato-bold";

/// The licence of the fonts the window ships, as shown under Help.
pub const FONT_LICENCE: &str = include_str!("../assets/fonts/OFL.txt");

const LATO_REGULAR: &str = "Lato-Regular";
const LATO_BOLD: &str = "Lato-Bold";

const REM: f32 = 16.0;
const FS_H2: f32 = 1.5 * REM;
const FS_BODY: f32 = REM;
const FS_SM: f32 = 0.9 * REM;
const FS_XS: f32 = 0.8 * REM;

/// Sets the KRT fonts and style on `ctx` and keeps it dark whatever the system theme.
pub fn apply(ctx: &Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(Theme::Dark);
    ctx.set_style_of(Theme::Dark, style());
}

/// Returns Lato Regular as the first font of every family and Lato Bold as [`BOLD_FAMILY`], with
/// egui's fonts behind them for glyphs Lato lacks.
#[must_use]
pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        LATO_REGULAR.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Lato-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        LATO_BOLD.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Lato-Bold.ttf"
        ))),
    );
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, LATO_REGULAR.to_owned());
    }
    let bold = std::iter::once(LATO_BOLD.to_owned())
        .chain(fallbacks)
        .collect();
    fonts
        .families
        .insert(FontFamily::Name(BOLD_FAMILY.into()), bold);
    fonts
}

/// Returns the egui style of the KRT design system.
#[must_use]
pub fn style() -> Style {
    let mut style = Style {
        visuals: visuals(),
        ..Style::default()
    };
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(FS_XS)),
        (TextStyle::Body, FontId::proportional(FS_BODY)),
        (TextStyle::Button, FontId::proportional(FS_SM)),
        (TextStyle::Monospace, FontId::monospace(FS_BODY)),
        (
            TextStyle::Heading,
            FontId::new(FS_H2, FontFamily::Name(BOLD_FAMILY.into())),
        ),
    ]
    .into();
    style
}

fn visuals() -> Visuals {
    let shadow = Shadow {
        offset: [0, 8],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(204),
    };
    Visuals {
        dark_mode: true,
        override_text_color: None,
        weak_text_color: Some(palette::GRAY_2_TEXT),
        widgets: widgets(),
        selection: Selection {
            bg_fill: palette::PRIMARY,
            stroke: Stroke::new(1.0, palette::BLACK),
        },
        hyperlink_color: palette::PRIMARY,
        faint_bg_color: palette::SURFACE_INPUT,
        extreme_bg_color: palette::BLACK,
        text_edit_bg_color: Some(palette::SURFACE_INPUT),
        code_bg_color: palette::SURFACE_INPUT,
        warn_fg_color: palette::WARNING,
        error_fg_color: palette::DANGER_TEXT,
        window_corner_radius: CornerRadius::ZERO,
        window_shadow: shadow,
        window_fill: palette::GRAY_4,
        window_stroke: Stroke::new(1.0, palette::GRAY_3),
        menu_corner_radius: CornerRadius::ZERO,
        panel_fill: palette::GRAY_4,
        popup_shadow: shadow,
        text_cursor: TextCursorStyle {
            stroke: Stroke::new(2.0, palette::PRIMARY),
            ..TextCursorStyle::default()
        },
        ..Visuals::dark()
    }
}

fn widgets() -> Widgets {
    let widget = |fill: Color32, outline: Color32, text: Color32| WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, outline),
        corner_radius: CornerRadius::ZERO,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };
    Widgets {
        noninteractive: widget(palette::GRAY_4, palette::GRAY_3, palette::GRAY_1),
        inactive: widget(palette::SURFACE_INPUT, palette::GRAY_3, palette::GRAY_1),
        hovered: widget(palette::GRAY_3, palette::PRIMARY, palette::WHITE),
        active: widget(palette::ACCENT_DARK, palette::PRIMARY, palette::WHITE),
        open: widget(palette::GRAY_3, palette::GRAY_3, palette::WHITE),
    }
}
