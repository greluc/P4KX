use egui::{Frame, Margin, Popup, PopupKind, Response, Stroke, TextStyle, Tooltip, Ui};

use super::frames::px;
use crate::theme::desktop::{TOOLTIP_FILL, TOOLTIP_PADDING_X, TOOLTIP_PADDING_Y, TOOLTIP_SIZE};
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
    use egui::CornerRadius;

    use super::frame;
    use crate::theme::desktop::TOOLTIP_FILL;
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
}
