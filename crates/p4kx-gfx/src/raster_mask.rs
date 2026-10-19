use std::panic::{AssertUnwindSafe, catch_unwind};

use tiny_skia::{FillRule, Mask, Path, PathStroker};

use crate::raster_paint::{stroke, transform};
use crate::scene::{Affine, LayerKind, Scene};
use crate::scene_report::SkipReason;

/// The bytes the masks of a rasterisation may hold at the same time.
const MAX_MASK_BYTES: u64 = 1 << 28;

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
}

impl Masks {
    /// Prepares the masks of `scene` for a canvas of `width` x `height` pixels.
    pub(crate) fn new(scene: &Scene, width: u32, height: u32) -> Self {
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
        }
    }

    /// Returns the mask `index`, building it and the masks around it first if needed.
    pub(crate) fn get(
        &mut self,
        scene: &Scene,
        paths: &[Vec<Option<Path>>],
        index: usize,
        root: &Affine,
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
            self.build(scene, paths, at, root)?;
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
    ) -> Result<(), SkipReason> {
        let bytes = u64::from(self.width) * u64::from(self.height);
        let total = self.live_bytes.saturating_add(bytes);
        if total > MAX_MASK_BYTES {
            return Err(SkipReason::MaskMemory);
        }
        let mut mask = Mask::new(self.width, self.height).ok_or(SkipReason::MaskMemory)?;
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
                let filled = catch_unwind(AssertUnwindSafe(|| match &layer.kind {
                    LayerKind::Fill => {
                        mask.fill_path(outline, FillRule::EvenOdd, true, to_canvas);
                    }
                    LayerKind::Stroke(style) => {
                        let scale = PathStroker::compute_resolution_scale(&to_canvas);
                        if let Some(area) = outline.stroke(&stroke(style), scale) {
                            mask.fill_path(&area, FillRule::Winding, true, to_canvas);
                        }
                    }
                }));
                if filled.is_err() {
                    return Err(SkipReason::Raster);
                }
            }
        }
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

/// Returns the parent of the mask `index`; a parent always comes before its child.
fn parent(scene: &Scene, index: usize) -> Option<usize> {
    scene
        .masks
        .get(index)
        .and_then(|mask| mask.parent)
        .filter(|&outer| outer < index)
}
