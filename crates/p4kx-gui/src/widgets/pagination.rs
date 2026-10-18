use std::sync::Arc;

use egui::{
    Align, Color32, CornerRadius, Galley, Layout, Pos2, Response, Role, Sense, Stroke, StrokeKind,
    Ui, WidgetInfo, pos2, vec2,
};

use super::button::focus_ring;
use crate::theme::desktop::{
    CHEVRON, CHEVRON_STROKE, FOCUS_OFFSET, PAGE_BUTTON_PADDING_X, PAGE_DISABLED_OPACITY, PAGE_GAP,
    PAGE_STATUS_PADDING_X, TRANSITION,
};
use crate::theme::palette;
use crate::theme::tokens::TOUCH_TARGET;

/// The accessible names of the page buttons and of the page status.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PageNames<'a> {
    pub(crate) first: &'a str,
    pub(crate) previous: &'a str,
    pub(crate) next: &'a str,
    pub(crate) last: &'a str,
    pub(crate) status: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    First,
    Previous,
    Next,
    Last,
}

/// Shows a `.pagination` for `page` of `pages`, counted from 0, with `status` as the laid-out
/// `.page-status`; returns the page a button asks for.
pub(crate) fn pagination(
    ui: &mut Ui,
    (page, pages): (usize, usize),
    status: Arc<Galley>,
    names: PageNames<'_>,
) -> Option<usize> {
    let last = pages.saturating_sub(1);
    let mut asked = None;
    let status_width = PAGE_STATUS_PADDING_X.mul_add(2.0, status.size().x);
    let buttons: f32 = [Step::First, Step::Previous, Step::Next, Step::Last]
        .into_iter()
        .map(button_width)
        .sum();
    let size = vec2(PAGE_GAP.mul_add(4.0, buttons + status_width), TOUCH_TARGET);
    let layout = Layout::left_to_right(Align::Center);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        ui.spacing_mut().item_spacing.x = PAGE_GAP;
        for (step, name, enabled) in [
            (Step::First, names.first, page > 0),
            (Step::Previous, names.previous, page > 0),
        ] {
            if page_button(ui, step, name, enabled).clicked() {
                asked = Some(if step == Step::First { 0 } else { page - 1 });
            }
        }
        let (rect, response) =
            ui.allocate_exact_size(vec2(status_width, TOUCH_TARGET), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, names.status));
        let at = pos2(
            rect.left() + PAGE_STATUS_PADDING_X,
            rect.center().y - status.size().y / 2.0,
        );
        ui.painter().galley(at, status, palette::GRAY_1);
        for (step, name, enabled) in [
            (Step::Next, names.next, page < last),
            (Step::Last, names.last, page < last),
        ] {
            if page_button(ui, step, name, enabled).clicked() {
                asked = Some(if step == Step::Last { last } else { page + 1 });
            }
        }
    });
    asked
}

fn button_width(step: Step) -> f32 {
    let icon = if matches!(step, Step::First | Step::Last) {
        CHEVRON * 1.5
    } else {
        CHEVRON
    };
    PAGE_BUTTON_PADDING_X.mul_add(2.0, icon).max(TOUCH_TARGET)
}

fn page_button(ui: &mut Ui, step: Step, name: &str, enabled: bool) -> Response {
    let double = matches!(step, Step::First | Step::Last);
    let width = button_width(step);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, TOUCH_TARGET), sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, name));
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id, enabled && response.hovered(), TRANSITION);
    let dim = |colour: Color32| {
        if enabled {
            colour
        } else {
            colour.gamma_multiply(PAGE_DISABLED_OPACITY)
        }
    };
    let border = dim(palette::CONTROL_BORDER.lerp_to_gamma(palette::PRIMARY, hover));
    let colour = dim(palette::CONTROL_FG.lerp_to_gamma(palette::PRIMARY, hover));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, dim(palette::CONTROL_BG));
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let left = matches!(step, Step::First | Step::Previous);
    let centre = rect.center();
    if double {
        let offset = CHEVRON / 4.0;
        chevron(ui, pos2(centre.x - offset, centre.y), left, colour);
        chevron(ui, pos2(centre.x + offset, centre.y), left, colour);
    } else {
        chevron(ui, centre, left, colour);
    }
    if response.has_focus() {
        focus_ring(ui, rect, palette::PRIMARY, FOCUS_OFFSET);
    }
    response
}

/// Paints a chevron of the chevron size centred on `centre`, pointing left or right.
fn chevron(ui: &Ui, centre: Pos2, left: bool, colour: Color32) {
    let (quarter, half) = (CHEVRON / 4.0, CHEVRON / 2.0);
    let tip = if left { -quarter } else { quarter };
    let stroke = Stroke::new(CHEVRON_STROKE.min(quarter), colour);
    let top = pos2(centre.x - tip, centre.y - half);
    let point = pos2(centre.x + tip, centre.y);
    let bottom = pos2(centre.x - tip, centre.y + half);
    ui.painter().line_segment([top, point], stroke);
    ui.painter().line_segment([point, bottom], stroke);
}
