use egui::{
    EventFilter, Id, Key, Modifiers, Role, ScrollArea, Sense, Ui, Vec2, WidgetInfo,
    scroll_area::ScrollAreaOutput, vec2,
};

use super::rows::focus_inside;

/// Where a scroll region stood in the last frame.
#[derive(Clone, Copy, Default)]
struct Stand {
    offset: Vec2,
    view: Vec2,
    content: Vec2,
}

/// Shows the scroll area that `show` makes of the one it is given as a focusable region: it takes
/// the keyboard focus with Tab, shows the inner focus ring, and scrolls with the arrow keys by
/// `step`, Page Up and Page Down by a page, Home and End to the ends.
///
/// `id` names the region and must differ between regions; `label` is its accessible name.
pub(crate) fn show<R>(
    ui: &mut Ui,
    (id, label): (Id, impl Fn() -> String),
    step: f32,
    area: ScrollArea,
    show: impl FnOnce(ScrollArea, &mut Ui) -> ScrollAreaOutput<R>,
) -> ScrollAreaOutput<R> {
    let stand_id = id.with("scroll-stand");
    let stand: Stand = ui
        .data(|data| data.get_temp::<Stand>(stand_id))
        .unwrap_or_default();
    let focus_id = id.with("scroll-focus");
    let region = ui.available_rect_before_wrap();
    let focus = ui.interact(region, focus_id, Sense::focusable_noninteractive());
    focus.widget_info(|| WidgetInfo::labeled(Role::ScrollView, true, label()));
    let mut area = area;
    if focus.has_focus() {
        let filter = EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            ..EventFilter::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(focus_id, filter));
        if let Some(offset) = keyboard_offset(ui, stand, step) {
            area = area.scroll_offset(offset);
        }
    }
    let output = show(area, ui);
    focus_inside(ui.painter(), output.inner_rect, &focus);
    ui.data_mut(|data| {
        *data.get_temp_mut_or_default::<Stand>(stand_id) = Stand {
            offset: output.state.clamped_offset(),
            view: output.inner_rect.size(),
            content: output.content_size,
        };
    });
    output
}

/// Returns where the keys pressed on the focused region scroll to, or `None` when none was.
fn keyboard_offset(ui: &Ui, stand: Stand, step: f32) -> Option<Vec2> {
    let page = (stand.view.y - step).max(step);
    let max = (stand.content - stand.view).max(Vec2::ZERO);
    let keys = [
        Key::ArrowDown,
        Key::ArrowUp,
        Key::ArrowRight,
        Key::ArrowLeft,
        Key::PageDown,
        Key::PageUp,
        Key::Home,
        Key::End,
    ];
    let mut offset = None;
    ui.input_mut(|input| {
        if !input.modifiers.is_none() {
            return;
        }
        for key in keys {
            if input.consume_key(Modifiers::NONE, key) {
                let at = offset.unwrap_or(stand.offset);
                offset = Some(match key {
                    Key::ArrowDown => at + vec2(0.0, step),
                    Key::ArrowUp => at - vec2(0.0, step),
                    Key::ArrowRight => at + vec2(step, 0.0),
                    Key::ArrowLeft => at - vec2(step, 0.0),
                    Key::PageDown => at + vec2(0.0, page),
                    Key::PageUp => at - vec2(0.0, page),
                    Key::Home => vec2(at.x, 0.0),
                    _ => vec2(at.x, max.y),
                });
            }
        }
    });
    offset.map(|offset| offset.clamp(Vec2::ZERO, max))
}
