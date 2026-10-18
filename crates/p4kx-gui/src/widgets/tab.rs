use egui::{
    CornerRadius, Rect, Response, Role, Sense, Stroke, StrokeKind, TextStyle, TextWrapMode, Ui,
    WidgetInfo, WidgetText, pos2, vec2,
};

use super::button::focus_ring;
use super::text::{caps, line, paint_left, plain};
use crate::theme::desktop::{
    ACTIVE_BAR, FOCUS_WIDTH, SEGMENT_FOCUS_OFFSET, SEGMENT_HEIGHT, SEGMENT_PADDING_X,
    TAB_BAR_PADDING_LEFT, TAB_PADDING_X, TAB_SIZE, TAB_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, TOUCH_TARGET, Weight};

/// Shows a `.tab-nav` with `labels`, `current` the selected index; returns the clicked index.
pub(crate) fn tab_bar(ui: &mut Ui, labels: &[String], current: usize) -> Option<usize> {
    let width = ui.available_width();
    let (bar, _) = ui.allocate_exact_size(vec2(width, TOUCH_TARGET), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(bar, CornerRadius::ZERO, palette::BG_DARK_GRAY);
    painter.hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, palette::GRAY_3),
    );
    let mut left = bar.left() + TAB_BAR_PADDING_LEFT;
    let mut clicked = None;
    for (index, label) in labels.iter().enumerate() {
        let galley = WidgetText::from(caps(label, TAB_SIZE, Weight::Bold, TAB_TRACKING))
            .into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextStyle::Button,
            );
        let rect = Rect::from_min_size(
            pos2(left, bar.top()),
            vec2(TAB_PADDING_X.mul_add(2.0, galley.size().x), bar.height()),
        );
        left = rect.right();
        let id = ui.scope_id().with(("tab", index));
        let response = ui.interact(rect, id, Sense::click());
        let active = index == current;
        response.widget_info(|| WidgetInfo::selected(Role::Tab, true, active, label));
        let colour = if active {
            palette::WHITE
        } else if response.hovered() {
            palette::GRAY_1
        } else {
            palette::GRAY_2_TEXT
        };
        ui.painter().galley(
            pos2(
                rect.left() + TAB_PADDING_X,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            colour,
        );
        if active {
            let underline =
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - ACTIVE_BAR), rect.max);
            ui.painter()
                .rect_filled(underline, CornerRadius::ZERO, palette::PRIMARY);
        }
        if response.has_focus() {
            focus_ring(ui, rect, palette::PRIMARY, -FOCUS_WIDTH);
        }
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// Shows the items of a `.krt-segmented--sm` group with their data kept as written; `active` is
/// the pressed item. Returns the clicked index.
pub(crate) fn segments<S: AsRef<str>>(
    ui: &mut Ui,
    items: &[S],
    active: Option<usize>,
) -> Option<usize> {
    let mut clicked = None;
    let spacing = std::mem::replace(&mut ui.spacing_mut().item_spacing.x, 0.0);
    let mut group = Rect::NOTHING;
    for (index, item) in items.iter().enumerate() {
        let pressed = active == Some(index);
        let response = segment(ui, item.as_ref(), pressed, index > 0);
        group = group.union(response.rect);
        if response.clicked() {
            clicked = Some(index);
        }
    }
    ui.spacing_mut().item_spacing.x = spacing;
    ui.painter().rect_stroke(
        group,
        CornerRadius::ZERO,
        Stroke::new(1.0, palette::CONTROL_BORDER),
        StrokeKind::Inside,
    );
    clicked
}

fn segment(ui: &mut Ui, item: &str, pressed: bool, divided: bool) -> Response {
    let weight = if pressed {
        Weight::Bold
    } else {
        Weight::Regular
    };
    let galley = line(ui, plain(item, FS_XS, weight), f32::INFINITY);
    let size = vec2(
        SEGMENT_PADDING_X.mul_add(2.0, galley.size().x),
        SEGMENT_HEIGHT,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, pressed, item));
    let hovered = response.hovered() && !pressed;
    let (fill, text) = if pressed {
        (palette::CONTROL_SELECTED_BG, palette::CONTROL_SELECTED_FG)
    } else if hovered {
        (palette::CONTROL_BG_HOVER, palette::WHITE)
    } else {
        (palette::CONTROL_BG, palette::CONTROL_FG)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, palette::CONTROL_BG);
    painter.rect_filled(rect, CornerRadius::ZERO, fill);
    if divided {
        painter.vline(
            rect.left() + 0.5,
            rect.y_range(),
            Stroke::new(1.0, palette::CONTROL_BORDER),
        );
    }
    if pressed {
        let border = palette::CONTROL_SELECTED_BORDER;
        painter.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        let underline = Rect::from_min_max(pos2(rect.left(), rect.bottom() - ACTIVE_BAR), rect.max);
        painter.rect_filled(underline, CornerRadius::ZERO, border);
    }
    paint_left(ui, rect.shrink2(vec2(SEGMENT_PADDING_X, 0.0)), galley, text);
    if response.has_focus() {
        focus_ring(ui, rect, palette::PRIMARY, SEGMENT_FOCUS_OFFSET);
    }
    response
}
