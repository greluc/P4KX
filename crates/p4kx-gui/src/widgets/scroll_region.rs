// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{
    EventFilter, Id, Key, Modifiers, Rect, Response, Role, ScrollArea, Sense, Ui, Vec2, WidgetInfo,
    scroll_area::ScrollAreaOutput, vec2,
};

use super::rows::focus_inside;

/// Where a scroll region stood in the last frame.
#[derive(Clone, Copy, Default)]
pub(crate) struct Stand {
    pub(crate) offset: Vec2,
    pub(crate) view: Vec2,
    pub(crate) content: Vec2,
}

/// A row a region scrolls to once, however often the region is shown: `serial` tells one request
/// from the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Jump {
    pub(crate) serial: u64,
    pub(crate) row: u64,
}

impl Jump {
    /// Returns the vertical offset that shows the row second from the top, with `stride` between
    /// rows, the first time `jump` is asked for in the region named `id`, and `None` afterwards or
    /// without a jump.
    pub(crate) fn offset(jump: Option<Self>, ui: &Ui, id: Id, stride: f32) -> Option<f32> {
        let jump = jump?;
        let applied_id = id.with("jump-applied");
        if ui.data(|data| data.get_temp::<u64>(applied_id)) == Some(jump.serial) {
            return None;
        }
        ui.data_mut(|data| data.insert_temp(applied_id, jump.serial));
        Some(row_offset(jump.row, stride))
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a row index fits the precision of a scroll offset"
)]
fn row_offset(row: u64, stride: f32) -> f32 {
    row.saturating_sub(1) as f32 * stride
}

/// A scroll region between [`begin`] and [`Region::finish`], for content that sets up its own
/// scroll areas.
pub(crate) struct Region {
    focus: Response,
    stand_id: Id,
    requested: Option<Vec2>,
}

impl Region {
    /// Returns where the keys pressed in this frame scroll the region to, or `None` when none was.
    pub(crate) const fn requested(&self) -> Option<Vec2> {
        self.requested
    }

    /// Ends the region after its content was shown in `inner_rect` with the scroll state `stand`.
    pub(crate) fn finish(self, ui: &Ui, inner_rect: Rect, stand: Stand) {
        focus_inside(ui.painter(), inner_rect, &self.focus);
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Stand>(self.stand_id) = stand);
    }
}

/// Makes the rest of `ui` a focusable region named `label`, like [`show`] does for a scroll area.
pub(crate) fn begin(ui: &Ui, (id, label): (Id, impl Fn() -> String), step: f32) -> Region {
    let stand_id = id.with("scroll-stand");
    let stand: Stand = ui
        .data(|data| data.get_temp::<Stand>(stand_id))
        .unwrap_or_default();
    let focus_id = id.with("scroll-focus");
    let region = ui.available_rect_before_wrap();
    let focus = ui.interact(region, focus_id, Sense::focusable_noninteractive());
    focus.widget_info(|| WidgetInfo::labeled(Role::ScrollView, true, label()));
    if focus.gained_focus() && !ui.clip_rect().intersects(region) {
        focus.scroll_to_me(None);
    }
    let requested = if focus.has_focus() {
        let filter = EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            ..EventFilter::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(focus_id, filter));
        keyboard_offset(ui, stand, step)
    } else {
        None
    };
    Region {
        focus,
        stand_id,
        requested,
    }
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
    let region = begin(ui, (id, label), step);
    let mut area = area;
    if let Some(offset) = region.requested() {
        area = area.scroll_offset(offset);
    }
    let output = show(area, ui);
    let stand = Stand {
        offset: output.state.clamped_offset(),
        view: output.inner_rect.size(),
        content: output.content_size,
    };
    region.finish(ui, output.inner_rect, stand);
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

#[cfg(test)]
mod tests {
    use super::row_offset;

    #[test]
    fn a_row_is_scrolled_to_the_second_line_of_the_view() {
        assert_eq!(row_offset(10, 20.0), 180.0, "row 10 of 20 px");
    }

    #[test]
    fn the_first_rows_stay_at_the_top() {
        assert_eq!(row_offset(1, 20.0), 0.0, "row 1");
        assert_eq!(row_offset(0, 20.0), 0.0, "row 0");
    }
}
