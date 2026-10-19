use egui::{
    Color32, CornerRadius, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2,
    vec2,
};

use super::button::focus_ring;
use super::text::{centred, line, paint_left, plain};
use crate::theme::desktop::{
    CHECKBOX_RADIUS, DISABLED_OPACITY, FOCUS_OFFSET, RADIO_DOT, ROW_HEIGHT, TOGGLE_GAP, TOGGLE_SIZE,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};

/// The state of a checkbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Check {
    Off,
    On,
    Mixed,
}

/// Returns the fill, border and mark colour of a toggle that is `on`.
pub(crate) const fn colours(on: bool, hovered: bool) -> (Color32, Color32, Color32) {
    match (on, hovered) {
        (true, _) => (palette::PRIMARY, palette::PRIMARY, palette::BLACK),
        (false, true) => (palette::CONTROL_BG, palette::PRIMARY, palette::BLACK),
        (false, false) => (palette::CONTROL_BG, palette::CONTROL_BORDER, palette::BLACK),
    }
}

/// Shows a checkbox centred in the whole available cell, which is its hit area; `label` is its
/// accessible name. Returns whether the user toggled it, to `check`'s opposite.
pub(crate) fn checkbox_cell(ui: &mut Ui, check: Check, label: &str) -> bool {
    let size = vec2(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let selected = check == Check::On;
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, selected, label));
    let box_rect = Rect::from_min_size(centred(rect, TOGGLE_SIZE), vec2(TOGGLE_SIZE, TOGGLE_SIZE));
    paint_checkbox(ui, box_rect, check, response.hovered(), true);
    if response.has_focus() {
        focus_ring(ui, box_rect, palette::PRIMARY, FOCUS_OFFSET);
    }
    response.clicked()
}

/// Shows a checkbox with `text` beside it; returns whether the user toggled `checked`.
pub(crate) fn checkbox(ui: &mut Ui, checked: &mut bool, text: &str) -> Response {
    checkbox_if(ui, checked, text, true)
}

/// Shows a checkbox with `text` beside it, dimmed and without a click when not `enabled`;
/// returns whether the user toggled `checked`.
pub(crate) fn checkbox_if(ui: &mut Ui, checked: &mut bool, text: &str, enabled: bool) -> Response {
    let (rect, box_rect, mut response) = allocate(ui, text, enabled);
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, enabled, *checked, text));
    let check = if *checked { Check::On } else { Check::Off };
    paint_checkbox(ui, box_rect, check, response.hovered(), enabled);
    label(ui, rect, text, enabled);
    if response.has_focus() {
        focus_ring(ui, box_rect, palette::PRIMARY, FOCUS_OFFSET);
    }
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response
}

/// Shows a radio with `text` beside it, in the selected state if `selected`.
pub(crate) fn radio(ui: &mut Ui, selected: bool, text: &str, enabled: bool) -> Response {
    let (rect, dot_rect, response) = allocate(ui, text, enabled);
    response.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, selected, text));
    let (fill, border, mark) = colours(selected, enabled && response.hovered());
    let dim = |colour: Color32| {
        if enabled {
            colour
        } else {
            colour.gamma_multiply(DISABLED_OPACITY)
        }
    };
    let painter = ui.painter();
    let centre = dot_rect.center();
    painter.circle(
        centre,
        TOGGLE_SIZE / 2.0,
        dim(fill),
        Stroke::new(1.0, dim(border)),
    );
    if selected {
        painter.circle_filled(centre, RADIO_DOT / 2.0, dim(mark));
    }
    label(ui, rect, text, enabled);
    if response.has_focus() {
        focus_ring(ui, dot_rect, palette::PRIMARY, FOCUS_OFFSET);
    }
    response
}

fn allocate(ui: &mut Ui, text: &str, enabled: bool) -> (Rect, Rect, Response) {
    let galley = line(ui, plain(text, FS_SM, Weight::Regular), f32::INFINITY);
    let width = TOGGLE_SIZE + TOGGLE_GAP + galley.size().x;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), sense);
    let toggle = Rect::from_min_size(
        pos2(rect.left(), rect.center().y - TOGGLE_SIZE / 2.0),
        vec2(TOGGLE_SIZE, TOGGLE_SIZE),
    );
    (rect, toggle, response)
}

fn label(ui: &Ui, rect: Rect, text: &str, enabled: bool) {
    let galley = line(ui, plain(text, FS_SM, Weight::Regular), f32::INFINITY);
    let text_rect = rect.with_min_x(rect.left() + TOGGLE_SIZE + TOGGLE_GAP);
    let colour = if enabled {
        palette::GRAY_1
    } else {
        palette::GRAY_1.gamma_multiply(DISABLED_OPACITY)
    };
    paint_left(ui, text_rect, galley, colour);
}

fn paint_checkbox(ui: &Ui, rect: Rect, check: Check, hovered: bool, enabled: bool) {
    let (fill, border, mark) = colours(check != Check::Off, enabled && hovered);
    let dim = |colour: Color32| {
        if enabled {
            colour
        } else {
            colour.gamma_multiply(DISABLED_OPACITY)
        }
    };
    let (fill, border, mark) = (dim(fill), dim(border), dim(mark));
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(CHECKBOX_RADIUS),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let stroke = Stroke::new(2.0, mark);
    let at = |x: f32, y: f32| {
        pos2(
            x.mul_add(rect.width(), rect.left()),
            y.mul_add(rect.height(), rect.top()),
        )
    };
    match check {
        Check::Off => {}
        Check::On => {
            painter.line_segment([at(0.25, 0.52), at(0.43, 0.7)], stroke);
            painter.line_segment([at(0.43, 0.7), at(0.76, 0.32)], stroke);
        }
        Check::Mixed => {
            let half = 5.0;
            let centre = rect.center();
            painter.line_segment(
                [
                    pos2(centre.x - half, centre.y),
                    pos2(centre.x + half, centre.y),
                ],
                stroke,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::colours;
    use crate::theme::palette;

    #[test]
    fn a_toggle_turns_orange_with_a_black_mark_when_on() {
        assert_eq!(
            colours(true, false),
            (palette::PRIMARY, palette::PRIMARY, palette::BLACK),
            "on"
        );
        assert_eq!(
            colours(false, false).1,
            palette::CONTROL_BORDER,
            "Grau 2 border when off"
        );
        assert_eq!(colours(false, true).1, palette::PRIMARY, "orange on hover");
    }
}
