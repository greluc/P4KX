use egui::{Context, CornerRadius, FontFamily, TextStyle, Theme};

use super::tokens::{CORNER_SM, RADIUS_SM, RADIUS_SM_POINTS};
use super::{BOLD_FAMILY, LATO_BOLD, LATO_REGULAR, apply, fonts, palette, style};

#[test]
fn the_style_is_dark_with_the_p4kx_surfaces() {
    let visuals = style().visuals;

    assert!(visuals.dark_mode, "dark mode");
    assert_eq!(visuals.panel_fill, palette::GRAY_4, "Grau 4 surface");
    assert_eq!(
        visuals.extreme_bg_color,
        palette::GRAY_4,
        "Grau 4 scroll track"
    );
    assert_eq!(visuals.text_color(), palette::GRAY_1, "Grau 1 body text");
    assert_eq!(
        visuals.weak_text_color(),
        palette::GRAY_2_TEXT,
        "muted text"
    );
}

#[test]
fn orange_marks_selection_links_and_focus() {
    let visuals = style().visuals;

    assert_eq!(
        visuals.selection.bg_fill,
        palette::PRIMARY,
        "selection fill"
    );
    assert_eq!(
        visuals.selection.stroke.color,
        palette::PRIMARY,
        "focused input border"
    );
    assert_eq!(visuals.hyperlink_color, palette::PRIMARY, "links");
    assert_eq!(visuals.text_cursor.stroke.color, palette::PRIMARY, "cursor");
    assert_eq!(
        visuals.widgets.hovered.bg_stroke.color,
        palette::PRIMARY,
        "hover outline"
    );
}

#[test]
fn errors_and_warnings_use_the_accessible_text_tints() {
    let visuals = style().visuals;

    assert_eq!(visuals.error_fg_color, palette::DANGER_TEXT, "error");
    assert_eq!(visuals.warn_fg_color, palette::WARNING, "warning");
}

#[test]
fn windows_and_menus_take_the_small_radius_and_the_scrollbar_stays_square() {
    let visuals = style().visuals;
    let widgets = &visuals.widgets;

    for (name, corner) in [
        ("window", visuals.window_corner_radius),
        ("menu", visuals.menu_corner_radius),
    ] {
        assert_eq!(corner, CORNER_SM, "{name} corners");
    }
    for (name, corner) in [
        ("noninteractive", widgets.noninteractive.corner_radius),
        ("inactive", widgets.inactive.corner_radius),
        ("hovered", widgets.hovered.corner_radius),
        ("active", widgets.active.corner_radius),
        ("open", widgets.open.corner_radius),
    ] {
        assert_eq!(corner, CornerRadius::ZERO, "{name} corners");
    }
}

#[test]
fn the_small_radius_is_four_points_in_both_forms() {
    assert!(
        (RADIUS_SM - f32::from(RADIUS_SM_POINTS)).abs() < f32::EPSILON,
        "radius-sm"
    );
}

#[test]
fn lato_comes_first_in_every_family() {
    let fonts = fonts();

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let names = fonts.families.get(&family).cloned().unwrap_or_default();
        assert_eq!(
            names.first().map(String::as_str),
            Some(LATO_REGULAR),
            "{family:?}"
        );
    }
    let bold = fonts.families.get(&FontFamily::Name(BOLD_FAMILY.into()));
    assert_eq!(
        bold.and_then(|names| names.first()).map(String::as_str),
        Some(LATO_BOLD),
        "bold family"
    );
}

#[test]
fn headings_are_set_in_lato_bold() {
    let style = style();

    let heading = style.text_styles.get(&TextStyle::Heading);

    assert_eq!(
        heading.map(|font| font.family.clone()),
        Some(FontFamily::Name(BOLD_FAMILY.into())),
        "heading family"
    );
}

#[test]
fn applying_the_theme_keeps_the_window_dark_on_a_light_system() {
    let ctx = Context::default();
    ctx.options_mut(|options| options.fallback_theme = Theme::Light);

    apply(&ctx);

    assert_eq!(ctx.theme(), Theme::Dark, "dark");
    assert_eq!(
        ctx.style_of(ctx.theme()).visuals.panel_fill,
        palette::GRAY_4,
        "P4KX style"
    );
}
