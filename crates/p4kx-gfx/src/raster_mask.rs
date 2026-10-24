// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::panic::{AssertUnwindSafe, catch_unwind};

use tiny_skia::{FillRule, Mask, Path, PathStroker};

use crate::raster_budget::{Coverage, covered_pixels};
use crate::raster_paint::{stroke, transform};
use crate::scene::{Affine, LayerKind, Scene, StrokeStyle};
use crate::scene_report::SkipReason;

/// The masks of a rasterisation, each built when its first item is drawn from the fills and
/// strokes of its shapes, within its parent, and dropped after its last item and the last item
/// of the masks inside it.
pub(crate) struct Masks {
    width: u32,
    height: u32,
    last_use: Vec<usize>,
    built: Vec<Option<Mask>>,
    live: Vec<usize>,
    live_bytes: u64,
    max_bytes: u64,
}

impl Masks {
    /// Prepares the masks of `scene` for a canvas of `width` x `height` pixels, which together
    /// may hold at most `max_bytes` bytes at the same time.
    pub(crate) fn new(scene: &Scene, width: u32, height: u32, max_bytes: u64) -> Self {
        let mut last_use = vec![0; scene.masks.len()];
        for (at, item) in scene.items.iter().enumerate() {
            let mut next = item.mask;
            while let Some(index) = next {
                let Some(slot) = last_use.get_mut(index) else {
                    break;
                };
                *slot = at;
                next = parent(scene, index);
            }
        }
        Self {
            width,
            height,
            last_use,
            built: (0..scene.masks.len()).map(|_| None).collect(),
            live: Vec::new(),
            live_bytes: 0,
            max_bytes,
        }
    }

    /// Returns the mask `index`, building it and the masks around it first if needed, and
    /// charging what it covers to `coverage`.
    pub(crate) fn get(
        &mut self,
        scene: &Scene,
        paths: &[Vec<Option<Path>>],
        index: usize,
        root: &Affine,
        coverage: &mut Coverage,
    ) -> Result<&Mask, SkipReason> {
        let mut chain = Vec::new();
        let mut next = Some(index);
        while let Some(at) = next {
            if self.built.get(at).is_none_or(Option::is_some) {
                break;
            }
            chain.push(at);
            next = parent(scene, at);
        }
        for &at in chain.iter().rev() {
            self.build(scene, paths, at, root, coverage)?;
        }
        self.built
            .get(index)
            .and_then(Option::as_ref)
            .ok_or(SkipReason::MaskMemory)
    }

    fn build(
        &mut self,
        scene: &Scene,
        paths: &[Vec<Option<Path>>],
        index: usize,
        root: &Affine,
        coverage: &mut Coverage,
    ) -> Result<(), SkipReason> {
        let bytes = u64::from(self.width) * u64::from(self.height);
        let total = self.live_bytes.saturating_add(bytes);
        if total > self.max_bytes {
            return Err(SkipReason::MaskMemory);
        }
        let parent_pass = if parent(scene, index).is_some() { 2 } else { 1 };
        if !coverage.charge(bytes * parent_pass) {
            return Err(SkipReason::FillBudget);
        }
        let mut mask = Mask::new(self.width, self.height).ok_or(SkipReason::MaskMemory)?;
        self.paint_shapes(scene, paths, index, root, coverage, &mut mask)?;
        if let Some(outer) = parent(scene, index) {
            let outer = self
                .built
                .get(outer)
                .and_then(Option::as_ref)
                .ok_or(SkipReason::MaskMemory)?;
            for (coverage, outer) in mask.data_mut().iter_mut().zip(outer.data()) {
                let product = (u16::from(*coverage) * u16::from(*outer) + 127) / 255;
                *coverage = u8::try_from(product).unwrap_or(u8::MAX);
            }
        }
        if let Some(slot) = self.built.get_mut(index) {
            *slot = Some(mask);
            self.live.push(index);
            self.live_bytes = total;
        }
        Ok(())
    }

    fn paint_shapes(
        &self,
        scene: &Scene,
        paths: &[Vec<Option<Path>>],
        index: usize,
        root: &Affine,
        coverage: &mut Coverage,
        mask: &mut Mask,
    ) -> Result<(), SkipReason> {
        let shapes = scene.masks.get(index).map_or(&[][..], |mask| &mask.shapes);
        for &(shape, ref to_stage) in shapes {
            let (Some(layers), Some(outlines)) = (scene.shapes.get(shape), paths.get(shape)) else {
                continue;
            };
            let Some(to_canvas) = transform(&root.then(to_stage)) else {
                continue;
            };
            for (layer, outline) in layers.layers.iter().zip(outlines) {
                let Some(outline) = outline else {
                    continue;
                };
                let scale = PathStroker::compute_resolution_scale(&to_canvas);
                let style = match &layer.kind {
                    LayerKind::Fill => None,
                    LayerKind::Stroke(style) => Some(hairline_as_pixel(style, scale)),
                };
                let Some(cost) =
                    covered_pixels(outline, to_canvas, style.as_ref(), self.width, self.height)
                else {
                    continue;
                };
                if !coverage.charge(cost) {
                    return Err(SkipReason::FillBudget);
                }
                let filled = catch_unwind(AssertUnwindSafe(|| {
                    if let Some(style) = &style {
                        if let Some(area) = outline.stroke(&stroke(style), scale) {
                            mask.fill_path(&area, FillRule::Winding, true, to_canvas);
                        }
                    } else {
                        mask.fill_path(outline, FillRule::EvenOdd, true, to_canvas);
                    }
                }));
                if filled.is_err() {
                    return Err(SkipReason::Raster);
                }
            }
        }
        Ok(())
    }

    /// Drops the masks no item after the item `at` needs.
    pub(crate) fn release(&mut self, at: usize) {
        let (built, last_use) = (&mut self.built, &self.last_use);
        let mut freed = 0;
        self.live.retain(|&index| {
            if last_use.get(index).is_some_and(|&last| last > at) {
                return true;
            }
            if let Some(slot) = built.get_mut(index) {
                *slot = None;
            }
            freed += 1;
            false
        });
        let bytes = u64::from(self.width) * u64::from(self.height);
        self.live_bytes = self.live_bytes.saturating_sub(freed * bytes);
    }
}

/// Returns `style`, with a hairline widened to one canvas pixel at the resolution `scale`.
pub(crate) fn hairline_as_pixel(style: &StrokeStyle, scale: f32) -> StrokeStyle {
    if style.width > 0.0 {
        return *style;
    }
    StrokeStyle {
        width: 1.0 / scale.max(f32::EPSILON),
        ..*style
    }
}

/// Returns the parent of the mask `index`; a parent always comes before its child.
pub(crate) fn parent(scene: &Scene, index: usize) -> Option<usize> {
    scene
        .masks
        .get(index)
        .and_then(|mask| mask.parent)
        .filter(|&outer| outer < index)
}
