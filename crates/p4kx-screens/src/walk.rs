// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use p4kx_dataforge::Guid;

use crate::bindings::ParamValues;
use crate::canvas::{Canvas, CanvasStore};
use crate::colour::Palette;
use crate::context::Context;
use crate::elements;
use crate::fit::fit;
use crate::geometry::Geometry;
use crate::layout::{Frame, Sibling, place};
use crate::limits::Limits;
use crate::look::Look;
use crate::paint::Area;
use crate::style::{CanvasStyles, Modifiers, StyleCache, Subject, TagPaths, resolve};
use crate::text::FontStyleInfo;
use crate::transform::Placement;
use crate::walk_instance::Override;
use crate::walk_measure::{MeasureKey, Probe};
use crate::widget::{CanvasSizing, Widget};

pub(crate) struct Walk<'s, 'a, 'f, 't> {
    pub(crate) store: &'s mut CanvasStore<'a, 'f>,
    pub(crate) slots: &'s [String],
    pub(crate) tags: &'s TagPaths,
    pub(crate) brand: Option<&'s str>,
    pub(crate) cx: &'s mut Context<'t>,
    pub(crate) stack: Vec<Guid>,
    pub(crate) widgets: usize,
    pub(crate) limits: Limits,
    pub(crate) budget: u64,
    pub(crate) styles: StyleCache,
    pub(crate) palettes: HashMap<Guid, Palette>,
    pub(crate) sheets: Vec<Rc<CanvasStyles>>,
    pub(crate) above: Vec<Subject>,
    pub(crate) chain: Vec<u64>,
    pub(crate) cache: HashMap<Guid, Rc<CanvasStyles>>,
    pub(crate) params: Vec<ParamValues>,
    pub(crate) resolved: HashMap<(Guid, usize, u64), Rc<Modifiers>>,
    pub(crate) measured: HashMap<MeasureKey, [Option<f32>; 2]>,
    pub(crate) measures: usize,
    pub(crate) instances: usize,
    pub(crate) templates: Vec<(Guid, usize)>,
    pub(crate) overrides: Vec<Override>,
    pub(crate) pixel: f32,
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
            chain: Vec::new(),
            cache: HashMap::new(),
            params: Vec::new(),
            resolved: HashMap::new(),
            measured: HashMap::new(),
            measures: 0,
            instances: 0,
            templates: Vec::new(),
            overrides: Vec::new(),
            pixel: 1.0,
        }
    }

    pub(crate) fn enter(
        &mut self,
        canvas: &Canvas,
        frame: Area,
        alpha: f32,
        depth: usize,
        sizing: CanvasSizing,
    ) {
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
        let fitted = fit(canvas, frame, sizing);
        let pixel = self.pixel;
        let entered = match fitted.scaled {
            Some((affine, scale)) if self.cx.painter.push_transform(&affine) => {
                self.pixel /= scale;
                Some(true)
            }
            Some(_) => None,
            None => Some(false),
        };
        if let Some(scaled) = entered {
            self.push_chain(&canvas.guid);
            let frame = Frame {
                content: fitted.layout,
                flex: None,
                centred: fitted.centred,
            };
            self.siblings(canvas, &canvas.roots, canvas.roots.len(), &frame, alpha, depth);
            self.chain.pop();
            if scaled {
                self.cx.painter.pop_transform();
            }
        } else {
            self.cx.report.note(
                "Canvas",
                "scale of the canvas is not finite: not drawn",
                &canvas.record_path,
            );
        }
        self.pixel = pixel;
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

    pub(crate) fn push_chain(&mut self, key: &impl Hash) {
        let mut hasher = DefaultHasher::new();
        self.chain.last().copied().unwrap_or(0).hash(&mut hasher);
        key.hash(&mut hasher);
        self.chain.push(hasher.finish());
    }

    pub(crate) fn style_of(&mut self, canvas: &Canvas, index: usize, widget: &Widget) -> Rc<Modifiers> {
        let key = (canvas.guid, index, self.chain.last().copied().unwrap_or(0));
        let plain = self.overrides.is_empty();
        if plain && let Some(known) = self.resolved.get(&key) {
            return Rc::clone(known);
        }
        let mut style = resolve(
            &self.sheets,
            widget,
            &self.above,
            self.tags,
            &mut self.budget,
        );
        self.apply_overrides(canvas, index, &mut style);
        let style = Rc::new(style);
        if plain {
            self.resolved.insert(key, Rc::clone(&style));
        }
        style
    }

    pub(crate) fn siblings(
        &mut self,
        canvas: &Canvas,
        indices: &[usize],
        own: usize,
        frame: &Frame<'_>,
        alpha: f32,
        depth: usize,
    ) {
        if self.widgets.saturating_add(indices.len()) > self.limits.widgets {
            self.cx.report.truncated = true;
            return;
        }
        let mut siblings: Vec<Sibling<'_>> = Vec::with_capacity(indices.len());
        for (position, &index) in indices.iter().enumerate() {
            let Some(widget) = canvas.widgets.get(index).filter(|widget| widget.active) else {
                continue;
            };
            let style = self.style_of(canvas, index, widget);
            if self.budget == 0 {
                self.cx.report.truncated = true;
                break;
            }
            let geometry = Geometry::of(widget, &style);
            let mut plan = self.plan_kids(canvas, index, widget);
            self.commit_kids(widget, &mut plan);
            let has_children = !plan.kids.is_empty() || widget.canvas.is_some();
            let probe = Probe {
                canvas,
                index,
                widget,
                style: &style,
                geometry: &geometry,
                kids: &plan.kids,
                has_children,
            };
            let content = self.content_of(&probe, frame.content, depth);
            siblings.push(Sibling {
                widget,
                index,
                has_children,
                content,
                geometry,
                style,
                own_kids: plan.own,
                kids: plan.kids,
                instance: position >= own,
            });
        }
        let mut placed_all = place(frame, &siblings);
        placed_all.sort_by_key(|placed| siblings.get(placed.at).map_or(0, |s| s.widget.layer));
        for placed in placed_all {
            let Some(sibling) = siblings.get(placed.at) else {
                continue;
            };
            if placed.auto {
                self.cx.report.note(
                    &sibling.widget.type_name,
                    "automatic size taken as the size of the parent",
                    &sibling.widget.name,
                );
            }
            self.widget(canvas, sibling, placed.area, alpha, depth);
        }
    }

    fn widget(
        &mut self,
        canvas: &Canvas,
        sibling: &Sibling<'_>,
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
        let widget = sibling.widget;
        let geometry = &sibling.geometry;
        self.widgets += 1;
        let style = &*sibling.style;
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
        self.note_approximations(widget, geometry);
        let moved = match Placement::of(area, geometry) {
            Placement::Plain => false,
            Placement::Flat => return,
            Placement::Moved(affine) => {
                if !self.cx.painter.push_transform(&affine) {
                    self.cx.report.note(
                        &widget.type_name,
                        "transform is not finite: not drawn",
                        &widget.name,
                    );
                    return;
                }
                true
            }
        };
        let font = self.font_of(widget, style);
        let mut look = Look::of(widget, style, self.pixel, font);
        look.bound_text = self.bound_text(canvas, sibling.index);
        elements::paint(self.cx, widget, &look, area, alpha);
        let clipped = widget.overflow.clips();
        if clipped {
            self.cx.painter.push_clip(area);
        }
        self.descend(canvas, sibling, area, alpha, depth);
        if clipped {
            self.cx.painter.pop_clip();
        }
        if moved {
            self.cx.painter.pop_transform();
        }
    }

    fn descend(
        &mut self,
        canvas: &Canvas,
        sibling: &Sibling<'_>,
        area: Area,
        alpha: f32,
        depth: usize,
    ) {
        let widget = sibling.widget;
        let padding = sibling.geometry.padding;
        let content = area.inset(padding.top, padding.right, padding.bottom, padding.left);
        let subject = Subject::of(&widget.type_name, &widget.tags);
        self.push_chain(&(&subject.kind, &subject.tags));
        self.above.push(subject);
        if sibling.instance {
            self.templates.push((canvas.guid, sibling.index));
        }
        let overridden = self.push_override(canvas, sibling);
        let frame = Frame {
            content,
            flex: sibling.geometry.flex.as_ref(),
            centred: false,
        };
        self.siblings(
            canvas,
            &sibling.kids,
            sibling.own_kids,
            &frame,
            alpha,
            depth + 1,
        );
        if let Some(nested) = elements::nested_canvas(widget) {
            let params = Rc::clone(&widget.params);
            self.nested(nested, params, content, widget.canvas_sizing, alpha, depth + 1);
        } else if widget.canvas_dangling {
            self.cx.report.missing_canvases.push(format!(
                "{} (a reference that no Record answers)",
                widget.name
            ));
        }
        if overridden {
            self.overrides.pop();
        }
        if sibling.instance {
            self.templates.pop();
        }
        self.above.pop();
        self.chain.pop();
    }

    pub(crate) fn font_of(
        &mut self,
        widget: &Widget,
        style: &Modifiers,
    ) -> Option<Rc<FontStyleInfo>> {
        let spec = widget.text.as_ref()?;
        let guid = style.text.font_style.or(spec.font_style)?;
        self.cx.texts.styles.get(self.store.forge(), guid)
    }

    pub(crate) fn bound_text(&self, canvas: &Canvas, index: usize) -> Option<String> {
        let binding = canvas.text_bindings.get(&index)?;
        let given = self.params.last().and_then(|values| {
            values
                .iter()
                .find(|(name, _)| *name == binding.parameter)
                .map(|(_, value)| value.clone())
        });
        given.or_else(|| binding.default.clone())
    }

    fn note_approximations(&mut self, widget: &Widget, geometry: &Geometry) {
        let report = &mut self.cx.report;
        if let Some(layout) = &widget.other_layout {
            report.note(
                &widget.type_name,
                &format!("{layout} children are laid out as an overlay"),
                &widget.name,
            );
        }
        if matches!(
            widget.overflow,
            crate::geometry::Overflow::Fade | crate::geometry::Overflow::ClipFade
        ) {
            report.note(
                &widget.type_name,
                "fading edges of the overflow are not drawn",
                &widget.name,
            );
        }
        if geometry.invalid {
            report.note(
                &widget.type_name,
                "a number of the geometry is not finite: the default is used",
                &widget.name,
            );
        }
    }

    fn nested(
        &mut self,
        guid: Guid,
        params: ParamValues,
        frame: Area,
        sizing: CanvasSizing,
        alpha: f32,
        depth: usize,
    ) {
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
        self.enter(&canvas, frame, alpha, depth, sizing);
        self.params.pop();
    }
}
