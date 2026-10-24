// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::Guid;

use crate::canvas::Canvas;
use crate::geometry::{Dim, Geometry};
use crate::look::Look;
use crate::paint::Area;
use crate::style::{Modifiers, Subject};
use crate::walk::Walk;
use crate::widget::Widget;

/// A widget measured under an ancestor chain, in a parent of a size, at a pixel size.
pub(crate) type MeasureKey = (Guid, usize, u64, [u32; 2], u32);

/// A widget whose content size is asked for, with what the cascade made of it.
pub(crate) struct Probe<'c> {
    pub(crate) canvas: &'c Canvas,
    pub(crate) index: usize,
    pub(crate) widget: &'c Widget,
    pub(crate) style: &'c Modifiers,
    pub(crate) geometry: &'c Geometry,
    pub(crate) kids: &'c [usize],
    pub(crate) has_children: bool,
}

fn edges(geometry: &Geometry, axis: usize) -> (f32, f32) {
    let (padding, margin) = (geometry.padding, geometry.margin);
    if axis == 0 {
        (padding.left + padding.right, margin.left + margin.right)
    } else {
        (padding.top + padding.bottom, margin.top + margin.bottom)
    }
}

impl Walk<'_, '_, '_, '_> {
    /// Returns the size along each axis that the content of a widget asks for where its size is
    /// automatic: a text as large as its glyphs, a container as large as its children. An axis
    /// without such a size is `None`.
    pub(crate) fn content_of(
        &mut self,
        probe: &Probe<'_>,
        parent: Area,
        depth: usize,
    ) -> [Option<f32>; 2] {
        if !probe.geometry.size.contains(&Dim::Auto) {
            return [None; 2];
        }
        let key = (
            probe.canvas.guid,
            probe.index,
            self.chain.last().copied().unwrap_or(0),
            [parent.width.to_bits(), parent.height.to_bits()],
            self.pixel.to_bits(),
        );
        if let Some(known) = self.measured.get(&key) {
            return *known;
        }
        let size = if depth > self.limits.depth || self.limits.widgets <= self.measures {
            [None; 2]
        } else {
            self.measures += 1;
            self.intrinsic(probe, parent, depth)
        };
        self.measured.insert(key, size);
        size
    }

    fn intrinsic(&mut self, probe: &Probe<'_>, parent: Area, depth: usize) -> [Option<f32>; 2] {
        if probe.has_children {
            return if probe.kids.is_empty() {
                [None; 2]
            } else {
                self.container_size(probe, parent, depth)
            };
        }
        let widget = probe.widget;
        if widget.text.is_none() || !self.cx.texts.is_enabled() {
            return [None; 2];
        }
        let font = self.font_of(widget, probe.style);
        let mut look = Look::of(widget, probe.style, self.pixel, font);
        look.bound_text = self.bound_text(probe.canvas, probe.index);
        let Some([width, height]) = self.cx.texts.measure(widget, &look, parent) else {
            return [None; 2];
        };
        let padding = probe.geometry.padding;
        [
            Some(width + padding.left + padding.right),
            Some(height + padding.top + padding.bottom),
        ]
    }

    fn container_size(&mut self, probe: &Probe<'_>, parent: Area, depth: usize) -> [Option<f32>; 2] {
        let subject = Subject::of(&probe.widget.type_name, &probe.widget.tags);
        self.push_chain(&(&subject.kind, &subject.tags));
        self.above.push(subject);
        let flex = probe.geometry.flex;
        let mut main = [0.0_f32; 2];
        let mut cross = [0.0_f32; 2];
        let mut seen = [0_usize; 2];
        let mut flowing = 0_usize;
        for &kid in probe.kids {
            let Some(widget) = probe.canvas.widgets.get(kid).filter(|w| w.active && w.in_flow)
            else {
                continue;
            };
            let style = self.style_of(probe.canvas, kid, widget);
            let geometry = Geometry::of(widget, &style);
            let kids = self.plan_kids(probe.canvas, kid, widget).kids;
            let child = Probe {
                canvas: probe.canvas,
                index: kid,
                widget,
                style: &style,
                geometry: &geometry,
                has_children: !kids.is_empty() || widget.canvas.is_some(),
                kids: &kids,
            };
            let content = self.content_of(&child, parent, depth + 1);
            flowing += 1;
            for axis in 0..2 {
                let own = match geometry.size.get(axis).copied() {
                    Some(Dim::Fixed(value)) => Some(value),
                    Some(Dim::Auto) => content.get(axis).copied().flatten(),
                    _ => None,
                };
                let Some(own) = own.filter(|own| own.is_finite()) else {
                    continue;
                };
                let (_, margin) = edges(&geometry, axis);
                let outer = own.max(0.0) + margin;
                seen[axis] += 1;
                main[axis] += outer;
                cross[axis] = cross[axis].max(outer);
            }
        }
        self.above.pop();
        self.chain.pop();
        let column = flex.is_some_and(|flex| flex.column);
        let gap = flex.map_or(0.0, |flex| if column { flex.row_gap } else { flex.column_gap });
        let spacing = gap * (flowing.saturating_sub(1) as f32);
        let mut size = [None; 2];
        for (axis, slot) in size.iter_mut().enumerate() {
            if seen.get(axis).copied().unwrap_or(0) == 0 {
                continue;
            }
            let along = flex.is_some() && (axis == 1) == column;
            let inner = if along {
                main[axis] + spacing
            } else {
                cross[axis]
            };
            let (padding, _) = edges(probe.geometry, axis);
            *slot = Some(inner + padding);
        }
        size
    }
}
