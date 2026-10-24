// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::rc::Rc;

use p4kx_dataforge::Guid;

use crate::bindings::ParamValues;
use crate::canvas::Canvas;
use crate::colour::Palette;
use crate::fit::fit;
use crate::layout::Frame;
use crate::paint::Area;
use crate::style::CanvasStyles;
use crate::walk::Walk;
use crate::widget::CanvasSizing;

impl Walk<'_, '_, '_, '_> {
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
        self.lay_out(canvas, frame, alpha, depth, sizing);
        self.cx.palette = inherited;
        self.sheets.pop();
        self.stack.pop();
    }

    /// Lays the root widgets of `canvas` out in `frame`, under the scale the canvas asks for.
    fn lay_out(
        &mut self,
        canvas: &Canvas,
        frame: Area,
        alpha: f32,
        depth: usize,
        sizing: CanvasSizing,
    ) {
        let fitted = fit(canvas, frame, sizing);
        let pixel = self.pixel;
        let scaled = match fitted.scaled {
            Some((affine, scale)) if self.cx.painter.push_transform(&affine) => {
                self.pixel /= scale;
                Some(true)
            }
            Some(_) => None,
            None => Some(false),
        };
        if let Some(scaled) = scaled {
            self.push_chain(&canvas.guid);
            let frame = Frame {
                content: fitted.layout,
                flex: None,
                centred: fitted.centred,
            };
            let roots = &canvas.roots;
            self.siblings(canvas, roots, roots.len(), &frame, alpha, depth);
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

    /// Enters the canvas `guid` that a canvas widget names, in the box `frame`.
    pub(crate) fn nested(
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
