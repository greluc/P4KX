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
                Found {
                    role: *role,
                    label: node.accesskit_node().label().unwrap_or_default(),
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

fn clip_problems(found: &Placed<'_>, regions: &ui::Regions, name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let scrolls = regions
        .work
        .is_some_and(|work| work.width() < WORK_MIN_WIDTH || work.height() < WORK_MIN_HEIGHT);
    let containers = [
        regions.list,
        regions.preview,
        regions.work.filter(|_| !scrolls),
    ];
    for (_, _, one) in found.iter().filter(|(_, _, one)| one.in_work) {
        for container in containers.iter().flatten() {
            if container.contains(one.rect.center())
                && !container.expand(1.0).contains_rect(one.rect)
            {
                problems.push(format!(
                    "{name}: `{}` {:?} is clipped by its region {container:?}",
                    one.label, one.rect
                ));
            }
        }
    }
    problems
}
