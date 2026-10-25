// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::scroll_area::{DragScroll, ScrollSource};
use egui::{ScrollArea, Sense, Ui, UiBuilder, Vec2, vec2};

use super::regions;
use crate::theme::window_metrics::{WORK_MIN_HEIGHT, WORK_MIN_WIDTH};

/// Shows `body` in the room below the tab bar. The room never gets smaller than
/// [`WORK_MIN_WIDTH`] by [`WORK_MIN_HEIGHT`] for it: a smaller window scrolls the work surface
/// instead of squeezing the controls of the tab into each other. What `body` lays out beyond
/// its room is clipped, as it is without the scrolling.
pub(super) fn show<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    let size = room(ui.available_size(), ui.spacing().scroll.allocated_width());
    let shown = ScrollArea::both()
        .id_salt("work-surface")
        .auto_shrink(false)
        .scroll_source(ScrollSource {
            drag: DragScroll::Never,
            ..ScrollSource::default()
        })
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            let id = child.unique_id();
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_role(egui::accesskit::Role::GenericContainer);
            });
            regions::record(ui.ctx(), |regions| regions.work_ui = Some(id));
            body(&mut child)
        });
    regions::record(ui.ctx(), |regions| regions.work = Some(shown.inner_rect));
    shown.inner
}

/// Returns the size `body` is laid out in when `available` is what is left below the tab bar and
/// a scroll bar takes `bar` of the other direction where one is needed.
fn room(available: Vec2, bar: f32) -> Vec2 {
    let mut vertical = available.y < WORK_MIN_HEIGHT;
    let horizontal = available.x - if vertical { bar } else { 0.0 } < WORK_MIN_WIDTH;
    vertical |= available.y - if horizontal { bar } else { 0.0 } < WORK_MIN_HEIGHT;
    vec2(
        (available.x - if vertical { bar } else { 0.0 }).max(WORK_MIN_WIDTH),
        (available.y - if horizontal { bar } else { 0.0 }).max(WORK_MIN_HEIGHT),
    )
}

#[cfg(test)]
mod tests {
    use egui::vec2;

    use super::room;
    use crate::theme::window_metrics::{WORK_MIN_HEIGHT, WORK_MIN_WIDTH};

    const BAR: f32 = 10.0;

    #[test]
    fn a_roomy_window_gives_the_tab_all_its_room() {
        let available = vec2(WORK_MIN_WIDTH + 200.0, WORK_MIN_HEIGHT + 100.0);

        assert_eq!(room(available, BAR), available, "all of it");
    }

    #[test]
    fn a_low_window_keeps_the_height_and_makes_room_for_the_vertical_bar() {
        let wide = vec2(WORK_MIN_WIDTH + 200.0, WORK_MIN_HEIGHT - 100.0);

        assert_eq!(
            room(wide, BAR),
            vec2(WORK_MIN_WIDTH + 200.0 - BAR, WORK_MIN_HEIGHT),
            "the minimum height, narrower by the bar"
        );
    }

    #[test]
    fn a_small_window_keeps_both_minimums() {
        let small = vec2(WORK_MIN_WIDTH - 100.0, WORK_MIN_HEIGHT - 100.0);

        assert_eq!(
            room(small, BAR),
            vec2(WORK_MIN_WIDTH, WORK_MIN_HEIGHT),
            "both minimums"
        );
    }

    #[test]
    fn a_window_just_wide_enough_scrolls_both_ways_once_the_bar_takes_its_width() {
        let edge = vec2(WORK_MIN_WIDTH + BAR / 2.0, WORK_MIN_HEIGHT - 1.0);

        assert_eq!(
            room(edge, BAR),
            vec2(WORK_MIN_WIDTH, WORK_MIN_HEIGHT),
            "the bar of the vertical scrolling makes it scroll sideways too"
        );
    }
}
