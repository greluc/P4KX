// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::rc::Rc;

use p4kx_dataforge::Guid;

use crate::bindings::ParamValues;
use crate::canvas::{Canvas, CanvasStore};
use crate::colour::Palette;
use crate::context::Context;
use crate::elements;
use crate::flex::Flex;
use crate::layout::{Sibling, place};
use crate::limits::Limits;
use crate::look::Look;
use crate::paint::Area;
use crate::style::{CanvasStyles, Modifiers, StyleCache, Subject, TagPaths, resolve};
use crate::text::FontStyleInfo;
use crate::widget::{Dim, Widget};

pub(crate) struct Walk<'s, 'a, 'f, 't> {
    pub(crate) store: &'s mut CanvasStore<'a, 'f>,
    pub(crate) slots: &'s [String],
    pub(crate) tags: &'s TagPaths,
    pub(crate) brand: Option<&'s str>,
    pub(crate) cx: &'s mut Context<'t>,
    pub(crate) stack: Vec<Guid>,
    pub(crate) widgets: usize,
    limits: Limits,
    budget: u64,
    styles: StyleCache,
    palettes: HashMap<Guid, Palette>,
    sheets: Vec<Rc<CanvasStyles>>,
    above: Vec<Subject>,
    cache: HashMap<Guid, Rc<CanvasStyles>>,
    params: Vec<ParamValues>,
}

impl<'s, 'a, 'f, 't> Walk<'s, 'a, 'f, 't> {
    pub(crate) fn new(
        store: &'s mut CanvasStore<'a, 'f>,
        slots: &'s [String],
        tags: &'s TagPaths,
        brand: Option<&'s str>,
        cx: &'s mut Context<'t>,
        limits: Limits,
    ) -> Self {
        Self {
            store,
            slots,
            tags,
            brand,
            cx,
            stack: Vec::new(),
            widgets: 0,
            limits,
            budget: limits.evaluations,
            styles: StyleCache::new(),
            palettes: HashMap::new(),
            sheets: Vec::new(),
            above: Vec::new(),
            cache: HashMap::new(),
            params: Vec::new(),
        }
    }

    pub(crate) fn enter(&mut self, canvas: &Canvas, frame: Area, alpha: f32, depth: usize) {
        self.cx.report.canvases += 1;
        self.stack.push(canvas.guid);
        let sheet = self.sheet(canvas);
        self.sheets.push(sheet);
        let inherited = std::mem::take(&mut self.cx.palette);
        self.cx.palette = Palette::of_canvas(
            self.store.forge(),
            self.slots,
            canvas,
            self.brand,
            &inherited,
            &mut self.palettes,
        );
        for &index in &canvas.unreachable {
            if let Some(widget) = canvas.widgets.get(index) {
                self.cx.report.note(
                    &widget.type_name,
                    "not reachable from a root: its parents form a cycle",
                    &widget.name,
                );
            }
        }
        self.siblings(canvas, &canvas.roots, None, frame, alpha, depth);
        self.cx.palette = inherited;
        self.sheets.pop();
        self.stack.pop();
    }

    fn sheet(&mut self, canvas: &Canvas) -> Rc<CanvasStyles> {
        if let Some(cached) = self.cache.get(&canvas.guid) {
            return Rc::clone(cached);
        }
        let sheet = Rc::new(CanvasStyles::build(
            self.store.forge(),
            &mut self.styles,
            canvas,
            self.brand,
        ));
        for (entry, selector) in sheet.unsupported() {
            self.cx.report.note(
                "StyleEntry",
                &format!("selector {selector} is not evaluated"),
                entry,
            );
        }
        self.cache.insert(canvas.guid, Rc::clone(&sheet));
        sheet
    }

    fn siblings(
        &mut self,
        canvas: &Canvas,
        indices: &[usize],
        flex: Option<&Flex>,
        content: Area,
        alpha: f32,
        depth: usize,
    ) {
        if self.widgets.saturating_add(indices.len()) > self.limits.widgets {
            self.cx.report.truncated = true;
            return;
        }
        let siblings: Vec<Sibling<'_>> = indices
            .iter()
            .filter_map(|&index| {
                let widget = canvas.widgets.get(index)?;
                let has_children = canvas
                    .children
                    .get(index)
                    .is_some_and(|children| !children.is_empty())
                    || widget.canvas.is_some();
                widget.active.then(|| Sibling {
                    widget,
                    index,
                    has_children,
                    content: self.content_of(canvas, index, has_children, content),
                })
            })
            .collect();
        let mut placed_all = place(content, flex, &siblings);
        placed_all.sort_by_key(|placed| {
            canvas
                .widgets
                .get(placed.index)
                .map_or(0, |widget| widget.layer)
        });
        for placed in placed_all {
            if let Some(widget) = canvas.widgets.get(placed.index) {
                if placed.auto {
                    self.cx.report.note(
                        &widget.type_name,
                        "automatic size taken as the size of the parent",
                        &widget.name,
                    );
                }
                self.widget(canvas, placed.index, widget, placed.area, alpha, depth);
            }
        }
    }

    fn widget(
        &mut self,
        canvas: &Canvas,
        index: usize,
        widget: &Widget,
        area: Area,
        alpha: f32,
        depth: usize,
    ) {
        if depth > self.limits.depth
            || self.widgets >= self.limits.widgets
            || self.cx.painter.is_full()
        {
            self.cx.report.truncated = true;
            return;
        }
        self.widgets += 1;
        let style = resolve(
            &self.sheets,
            widget,
            &self.above,
            self.tags,
            &mut self.budget,
        );
        if self.budget == 0 {
            self.cx.report.truncated = true;
            return;
        }
        if style.active == Some(false) {
            return;
        }
        self.cx.report.widgets += 1;
        let own = style.alpha.unwrap_or(widget.alpha);
        let alpha = if widget.inherits_alpha {
            alpha * own
        } else {
            own
        };
        self.note_approximations(widget);
        let font = self.font_of(widget, &style);
        let mut look = Look::of(widget, &style, canvas.unit, font);
        look.bound_text = self.bound_text(canvas, index);
        elements::paint(self.cx, widget, &look, area, alpha);
        let content = area.inset(
            widget.padding.top,
            widget.padding.right,
            widget.padding.bottom,
            widget.padding.left,
        );
        self.above
            .push(Subject::of(&widget.type_name, &widget.tags));
        let children = canvas.children.get(index).map_or(&[][..], Vec::as_slice);
        self.siblings(
            canvas,
            children,
            widget.flex.as_ref(),
            content,
            alpha,
            depth + 1,
        );
        if let Some(nested) = elements::nested_canvas(widget) {
            self.nested(nested, Rc::clone(&widget.params), content, alpha, depth + 1);
        } else if widget.canvas_dangling {
            self.cx.report.missing_canvases.push(format!(
                "{} (a reference that no Record answers)",
                widget.name
            ));
        }
        self.above.pop();
    }

    fn font_of(&mut self, widget: &Widget, style: &Modifiers) -> Option<Rc<FontStyleInfo>> {
        let spec = widget.text.as_ref()?;
        let guid = style.text.font_style.or(spec.font_style)?;
        self.cx.texts.styles.get(self.store.forge(), guid)
    }

    fn bound_text(&self, canvas: &Canvas, index: usize) -> Option<String> {
        let binding = canvas.text_bindings.get(&index)?;
        let given = self.params.last().and_then(|values| {
            values
                .iter()
                .find(|(name, _)| *name == binding.parameter)
                .map(|(_, value)| value.clone())
        });
        given.or_else(|| binding.default.clone())
    }

    fn content_of(
        &mut self,
        canvas: &Canvas,
        index: usize,
        has_children: bool,
        parent: Area,
    ) -> Option<[f32; 2]> {
        let widget = canvas.widgets.get(index)?;
        if has_children
            || widget.text.is_none()
            || !self.cx.texts.is_enabled()
            || !widget.size.contains(&Dim::Auto)
        {
            return None;
        }
        let style = resolve(
            &self.sheets,
            widget,
            &self.above,
            self.tags,
            &mut self.budget,
        );
        let font = self.font_of(widget, &style);
        let mut look = Look::of(widget, &style, canvas.unit, font);
        look.bound_text = self.bound_text(canvas, index);
        let [width, height] = self.cx.texts.measure(widget, &look, parent)?;
        Some([
            width + widget.padding.left + widget.padding.right,
            height + widget.padding.top + widget.padding.bottom,
        ])
    }

    fn note_approximations(&mut self, widget: &Widget) {
        if widget.rotated {
            self.cx
                .report
                .note(&widget.type_name, "rotation is not applied", &widget.name);
        }
        if let Some(layout) = &widget.other_layout {
            self.cx.report.note(
                &widget.type_name,
                &format!("{layout} children are laid out as an overlay"),
                &widget.name,
            );
        }
        if widget.flex.is_some_and(|flex| flex.wrap) {
            self.cx.report.note(
                &widget.type_name,
                "wrapping flex containers are laid out without wrapping",
                &widget.name,
            );
        }
    }

    fn nested(&mut self, guid: Guid, params: ParamValues, frame: Area, alpha: f32, depth: usize) {
        if self.stack.contains(&guid) {
            self.cx
                .report
                .missing_canvases
                .push(format!("{guid} (a canvas that contains itself)"));
            return;
        }
        let Some(canvas) = self.store.get(guid) else {
            self.cx.report.missing_canvases.push(guid.to_string());
            return;
        };
        self.params.push(params);
        self.enter(&canvas, frame, alpha, depth);
        self.params.pop();
    }
}
