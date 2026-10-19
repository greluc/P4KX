use egui::{Frame, Margin, Popup, PopupKind, Response, RichText, Stroke, TextStyle, Tooltip, Ui};

use super::frames::px;
use crate::theme::desktop::{
    TOOLTIP_FILL, TOOLTIP_LINE_HEIGHT, TOOLTIP_PADDING_X, TOOLTIP_PADDING_Y, TOOLTIP_SIZE,
};
use crate::theme::palette;
use crate::theme::tokens::{GLOW_TOOLTIP, Weight, font};

/// The frame of a `.scu-hint__bubble`: near-black with an orange hairline and glow, square.
pub(crate) fn frame() -> Frame {
    Frame::NONE
        .fill(TOOLTIP_FILL)
        .stroke(Stroke::new(1.0, palette::PRIMARY))
        .shadow(GLOW_TOOLTIP)
        .inner_margin(Margin::symmetric(
            px(TOOLTIP_PADDING_X.round()),
            px(TOOLTIP_PADDING_Y),
        ))
}

/// Shows a line of a tooltip with the design system's line height of 1.4, for a tooltip given to
/// [`hover`] or [`show`].
pub(crate) fn line(ui: &mut Ui, text: impl Into<RichText>) -> Response {
    ui.label(
        text.into()
            .line_height(Some(TOOLTIP_SIZE * TOOLTIP_LINE_HEIGHT)),
    )
}

fn style(ui: &mut Ui) {
    ui.style_mut()
        .text_styles
        .insert(TextStyle::Body, font(TOOLTIP_SIZE, Weight::Regular));
}

/// Shows `content` as the tooltip of `response` while it is hovered; returns `response`.
pub(crate) fn hover(response: Response, content: impl FnOnce(&mut Ui)) -> Response {
    let mut tooltip = Tooltip::for_enabled(&response);
    tooltip.popup = tooltip.popup.frame(frame());
    tooltip.show(|ui| {
        style(ui);
        content(ui);
    });
    response
}

/// Shows `content` as the tooltip of `response` whether it is hovered or not.
pub(crate) fn show(response: &Response, content: impl FnOnce(&mut Ui)) {
    Popup::from_response(response)
        .kind(PopupKind::Tooltip)
        .frame(frame())
        .show(|ui| {
            style(ui);
            content(ui);
        });
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use egui::CornerRadius;
    use egui_kittest::Harness;

    use super::{frame, line};
    use crate::theme::desktop::{TOOLTIP_FILL, TOOLTIP_LINE_HEIGHT, TOOLTIP_SIZE};
    use crate::theme::palette;
    use crate::theme::tokens::GLOW_TOOLTIP;

    #[test]
    fn a_tooltip_is_square_near_black_with_an_orange_hairline_and_glow() {
        let frame = frame();

        assert_eq!(frame.corner_radius, CornerRadius::ZERO, "square");
        assert_eq!(frame.fill, TOOLTIP_FILL, "fill");
        assert_eq!(frame.stroke.color, palette::PRIMARY, "hairline");
        assert_eq!(frame.shadow, GLOW_TOOLTIP, "glow");
    }

    #[test]
    fn a_tooltip_line_is_one_point_four_times_its_font_size_high() {
        let heights = Rc::new(Cell::new((0.0_f32, 0.0_f32)));
        let seen = Rc::clone(&heights);
        let mut harness = Harness::builder()
            .with_context_setup(crate::theme::apply)
            .build_ui(move |ui| {
                let first = line(ui, "Data\\Libs\\a.xml");
                let second = line(ui, "Item");
                seen.set((first.rect.height(), second.rect.height()));
            });
        harness.run();

        let want = TOOLTIP_SIZE * TOOLTIP_LINE_HEIGHT;
        let (first, second) = heights.get();
        assert!(
            (first - want).abs() < 0.5,
            "first line {first}, want {want}"
        );
        assert!(
            (second - want).abs() < 0.5,
            "second line {second}, want {want}"
        );
    }
}
