// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! What the layout tests check of a window: its main regions do not intersect, its controls do
//! not overlap as far as a scroll area shows them, and what a region holds is not clipped by it.

use egui::accesskit::Role;
use egui::{Id, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::theme::window_metrics::{WORK_MIN_HEIGHT, WORK_MIN_WIDTH};
use p4kx_gui::{Model, ui};

use crate::node_text::node_text;

const WIDGETS: [Role; 7] = [
    Role::Button,
    Role::TextInput,
    Role::MultilineTextInput,
    Role::RadioButton,
    Role::ComboBox,
    Role::Tab,
    Role::Label,
];

struct Found {
    role: Role,
    label: String,
    rect: Rect,
    seen: Rect,
    in_work: bool,
}

type Placed<'a> = Vec<(&'a str, Rect, Found)>;

fn widgets(harness: &Harness<'_, Model>) -> Vec<Found> {
    let regions = ui::regions(&harness.ctx);
    let named = |id: Id| format!("{:?})", id.accesskit_id());
    let work = regions.work_ui.map(named);
    let work_shown = regions.work;
    let scrolls = work_scrolls(work_shown);
    let clips: Vec<(String, Rect)> = regions
        .clips
        .iter()
        .map(|(id, rect)| (named(*id), *rect))
        .collect();
    WIDGETS
        .iter()
        .flat_map(|role| {
            let (work, clips) = (&work, &clips);
            harness.query_all_by_role(*role).map(move |node| {
                let mut in_work = false;
                let mut seen = node.rect();
                let mut up = node.parent();
                while let Some(parent) = up {
                    let id = format!("{:?}", parent.accesskit_node().id());
                    if parent.accesskit_node().role() == Role::ScrollView {
                        seen = seen.intersect(parent.rect());
                    }
                    in_work |= work.as_ref().is_some_and(|work| id.ends_with(work));
                    for (clip, rect) in clips {
                        if id.ends_with(clip) {
                            seen = seen.intersect(*rect);
                        }
                    }
                    up = parent.parent();
                }
                if let (true, Some(shown)) = (in_work && scrolls, work_shown) {
                    seen = seen.intersect(shown);
                }
                Found {
                    role: *role,
                    label: node_text(&node).unwrap_or_default(),
                    rect: node.rect(),
                    seen,
                    in_work,
                }
            })
        })
        .filter(|found| {
            found.rect.width() > 0.0
                && found.rect.height() > 0.0
                && (found.role != Role::Label || !found.label.is_empty())
        })
        .collect()
}

fn overlap(a: Rect, b: Rect) -> bool {
    let shared = a.intersect(b);
    shared.width() > 1.0 && shared.height() > 1.0
}

fn disclosure(label: &str) -> bool {
    ["Expand", "Collapse", "aufklappen", "zuklappen"]
        .iter()
        .any(|word| label.contains(word))
}

fn nested(outer: &Found, inner: &Found) -> bool {
    outer.rect.expand(1.0).contains_rect(inner.rect)
        && (inner.role == Role::Label || disclosure(&inner.label))
}

/// Returns what is wrong with the layout of `harness`, each problem naming the window `name`.
pub(super) fn problems(harness: &Harness<'_, Model>, name: &str) -> Vec<String> {
    let regions = ui::regions(&harness.ctx);
    let mut problems = region_problems(&regions, name);
    let areas = [
        ("rail", regions.rail),
        ("sidebar", regions.sidebar),
        ("header", regions.header),
        ("status bar", regions.status),
        ("tab bar", regions.tabs),
    ];
    let found: Placed<'_> = widgets(harness)
        .into_iter()
        .filter_map(|widget| {
            if widget.in_work {
                return Some(("work surface", regions.work?, widget));
            }
            let (area, rect) = areas.iter().find_map(|(area, rect)| {
                rect.filter(|rect| rect.contains(widget.rect.center()))
                    .map(|rect| (*area, rect))
            })?;
            Some((area, rect, widget))
        })
        .collect();
    problems.extend(overlap_problems(&found, name));
    problems.extend(clip_problems(&found, &regions, name));
    problems
}

fn region_problems(regions: &ui::Regions, name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let named = [
        ("rail", regions.rail),
        ("sidebar", regions.sidebar),
        ("work surface", regions.work),
    ];
    for (at, (first, a)) in named.iter().enumerate() {
        for (second, b) in named.iter().skip(at + 1) {
            if let (Some(a), Some(b)) = (a, b)
                && overlap(*a, *b)
            {
                problems.push(format!(
                    "{name}: the {first} {a:?} overlaps the {second} {b:?}"
                ));
            }
        }
    }
    if let (Some(list), Some(preview)) = (regions.list, regions.preview)
        && overlap(list, preview)
    {
        problems.push(format!(
            "{name}: the list {list:?} overlaps the preview {preview:?}"
        ));
    }
    problems
}

fn overlap_problems(found: &Placed<'_>, name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for (at, (area, _, one)) in found.iter().enumerate() {
        for (other_area, _, other) in found.iter().skip(at + 1) {
            if area == other_area
                && overlap(one.seen, other.seen)
                && !nested(one, other)
                && !nested(other, one)
            {
                problems.push(format!(
                    "{name}: `{}` {:?} overlaps `{}` {:?}",
                    one.label, one.rect, other.label, other.rect
                ));
            }
        }
    }
    problems
}

fn work_scrolls(work: Option<Rect>) -> bool {
    work.is_some_and(|work| work.width() < WORK_MIN_WIDTH || work.height() < WORK_MIN_HEIGHT)
}

fn clip_problems(found: &Placed<'_>, regions: &ui::Regions, name: &str) -> Vec<String> {
    let work = regions.work.and_then(|work| {
        if !work_scrolls(regions.work) {
            Some(work)
        } else if work.width() >= WORK_MIN_WIDTH {
            Some(Rect::from_x_y_ranges(
                work.x_range(),
                f32::NEG_INFINITY..=f32::INFINITY,
            ))
        } else {
            None
        }
    });
    clipped(found, [regions.list, regions.preview], work, name)
}

fn clipped(
    found: &Placed<'_>,
    shown_in: [Option<Rect>; 2],
    whole_in: Option<Rect>,
    name: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (_, _, one) in found
        .iter()
        .filter(|(_, _, one)| one.in_work && one.seen.width() > 0.0 && one.seen.height() > 0.0)
    {
        let checks = shown_in
            .iter()
            .flatten()
            .map(|container| (container, one.seen))
            .chain(whole_in.iter().map(|container| (container, one.rect)));
        for (container, shape) in checks {
            if container.contains(shape.center()) && !container.expand(1.0).contains_rect(shape) {
                problems.push(format!(
                    "{name}: `{}` {:?} is clipped by its region {container:?}",
                    one.label, one.rect
                ));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use egui::{Rect, pos2};

    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    }

    fn widget(label: &str, rect: Rect, seen: Rect) -> Placed<'static> {
        vec![(
            "work surface",
            rect,
            Found {
                role: Role::Button,
                label: label.to_owned(),
                rect,
                seen,
                in_work: true,
            },
        )]
    }

    #[test]
    fn a_visible_widget_that_overflows_a_work_surface_that_does_not_scroll_is_reported() {
        let work = rect(0.0, 0.0, 100.0, 100.0);
        let button = rect(80.0, 10.0, 120.0, 30.0);
        let found = widget("Overflowing", button, button);

        let problems = clipped(&found, [None, None], Some(work), "w");

        assert_eq!(problems.len(), 1, "{problems:?}");
    }

    #[test]
    fn a_widget_inside_the_work_surface_is_not_reported() {
        let work = rect(0.0, 0.0, 100.0, 100.0);
        let button = rect(10.0, 10.0, 50.0, 30.0);
        let found = widget("Inside", button, button);

        assert_eq!(
            clipped(&found, [None, None], Some(work), "w"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_work_surface_that_scrolls_is_told_by_its_size() {
        assert!(work_scrolls(Some(rect(0.0, 0.0, 10.0, 10.0))));
        assert!(!work_scrolls(Some(rect(
            0.0,
            0.0,
            WORK_MIN_WIDTH,
            WORK_MIN_HEIGHT
        ))));
        assert!(!work_scrolls(None));
    }

    #[test]
    fn a_widget_scrolled_out_of_a_marked_table_is_skipped() {
        let list = rect(0.0, 0.0, 100.0, 50.0);
        let row = rect(10.0, 60.0, 50.0, 80.0);
        let hidden = rect(10.0, 60.0, 10.0, 50.0);
        let found = widget("Row", row, hidden);

        assert_eq!(
            clipped(&found, [Some(list), None], None, "w"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_partly_scrolled_row_is_judged_by_the_part_that_shows() {
        let list = rect(0.0, 0.0, 100.0, 50.0);
        let row = rect(10.0, 45.0, 50.0, 65.0);
        let shown = rect(10.0, 45.0, 50.0, 50.0);
        let found = widget("Row", row, shown);

        assert_eq!(
            clipped(&found, [Some(list), None], None, "w"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_visible_overlap_is_still_reported() {
        let one = rect(0.0, 0.0, 50.0, 20.0);
        let other = rect(40.0, 0.0, 90.0, 20.0);
        let mut found = widget("One", one, one);
        found.extend(widget("Other", other, other));

        assert_eq!(overlap_problems(&found, "w").len(), 1);
    }

    #[test]
    fn a_widget_hidden_behind_another_is_not_an_overlap() {
        let one = rect(0.0, 0.0, 50.0, 20.0);
        let other = rect(40.0, 0.0, 90.0, 20.0);
        let hidden = rect(40.0, 0.0, 40.0, 0.0);
        let mut found = widget("One", one, one);
        found.extend(widget("Other", other, hidden));

        assert_eq!(overlap_problems(&found, "w"), Vec::<String>::new());
    }
}
