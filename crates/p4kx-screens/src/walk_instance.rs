// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::Guid;

use crate::canvas::Canvas;
use crate::layout::Sibling;
use crate::style::{Modifiers, merge};
use crate::walk::Walk;
use crate::widget::{CloneModifier, Widget};

const CLONE: &str = "BuildingBlocks_WidgetClone";
const LIST: &str = "BuildingBlocks_WidgetList";

const NO_TARGET: &str = "no target that is a widget of the canvas: nothing repeated";
const CYCLE: &str = "the template contains a clone of itself: not repeated";
const AT_REST: &str = "list rows come from run-time data: none drawn at rest";
const FROM_STATIC: &str = "list rows repeat the template without the values of their rows";

/// The modifiers of a clone, in force while the widget it instantiates is laid out.
#[derive(Debug)]
pub(crate) struct Override {
    canvas: Guid,
    root: usize,
    items: Vec<CloneModifier>,
}

/// The children of a widget: those of its subtree, then the instances of its template.
#[derive(Debug, Default)]
pub(crate) struct KidsPlan {
    pub(crate) kids: Vec<usize>,
    pub(crate) own: usize,
    repeats: usize,
    note: Option<&'static str>,
    cut: bool,
}

impl Walk<'_, '_, '_, '_> {
    /// Returns the children of widget `index`: those whose parent it is, and for a clone its
    /// template once, for a list its template once per row of its static array.
    pub(crate) fn plan_kids(&self, canvas: &Canvas, index: usize, widget: &Widget) -> KidsPlan {
        let kids = canvas.children.get(index).cloned().unwrap_or_default();
        let mut plan = KidsPlan {
            own: kids.len(),
            kids,
            ..KidsPlan::default()
        };
        let is_list = widget.type_name == LIST;
        if !is_list && widget.type_name != CLONE {
            return plan;
        }
        let Some(at) = widget
            .target
            .and_then(|target| canvas.target_index(target))
        else {
            plan.note = Some(NO_TARGET);
            return plan;
        };
        if self.templates.contains(&(canvas.guid, at)) {
            plan.note = Some(CYCLE);
            return plan;
        }
        let rows = if is_list {
            let known = widget
                .array_variable
                .as_deref()
                .and_then(|name| canvas.arrays.get(name));
            match known {
                Some(&rows) => {
                    plan.cut = rows > self.limits.rows;
                    rows.min(self.limits.rows)
                }
                None => {
                    plan.note = Some(AT_REST);
                    0
                }
            }
        } else {
            1
        };
        plan.kids.extend(std::iter::repeat_n(at, rows));
        plan.repeats = rows;
        plan
    }

    /// Counts the instances `plan` makes against the limit and reports what it left out.
    pub(crate) fn commit_kids(&mut self, widget: &Widget, plan: &mut KidsPlan) {
        if let Some(reason) = plan.note {
            self.cx.report.note(&widget.type_name, reason, &widget.name);
        }
        self.cx.report.truncated |= plan.cut;
        if plan.repeats == 0 {
            return;
        }
        if self.instances.saturating_add(plan.repeats) > self.limits.instances {
            self.cx.report.truncated = true;
            plan.kids.truncate(plan.own);
            plan.repeats = 0;
            return;
        }
        self.instances += plan.repeats;
        if widget.type_name == LIST {
            self.cx
                .report
                .note(&widget.type_name, FROM_STATIC, &widget.name);
        }
    }

    /// Puts the modifiers of a clone in force for its template; `true` when it did.
    pub(crate) fn push_override(&mut self, canvas: &Canvas, sibling: &Sibling<'_>) -> bool {
        let items = &sibling.widget.clone_modifiers;
        let root = sibling.kids.last().copied();
        let (false, true, Some(root)) = (
            items.is_empty(),
            sibling.kids.len() > sibling.own_kids,
            root,
        ) else {
            return false;
        };
        self.overrides.push(Override {
            canvas: canvas.guid,
            root,
            items: items.clone(),
        });
        true
    }

    /// Lets the modifiers of the clones in force replace what the cascade made of widget `index`.
    pub(crate) fn apply_overrides(&self, canvas: &Canvas, index: usize, style: &mut Modifiers) {
        for active in &self.overrides {
            for item in &active.items {
                let hit = match item.target {
                    None => active.canvas == canvas.guid && active.root == index,
                    Some(target) => {
                        active.canvas == canvas.guid && canvas.target_index(target) == Some(index)
                    }
                };
                if hit {
                    merge(style, &item.modifiers);
                }
            }
        }
    }
}
