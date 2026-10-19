use std::collections::HashMap;

use crate::assets::Assets;
use crate::geometry::Rect;
use crate::morph::MorphShape;
use crate::morph_frame::morph_at;
use crate::place::{ColorTransform, Placement};
use crate::scene::{
    Affine, MAX_SCENE_MASKS, MAX_SCENE_PATHS, MAX_SCENE_PLACEMENTS, Origin, Scene, SceneItem,
    SceneMask, SceneShape,
};
use crate::scene_report::{SkipReason, Skipped};
use crate::scene_shape::ShapeBuilder;
use crate::shape::Shape;
use crate::timeline::Sprite;
use crate::walker::MAX_SPRITE_DEPTH;

const TAG_DEFINE_BITS: u16 = 6;
const TAG_DEFINE_FONT: u16 = 10;

/// A definition a placement may name.
#[derive(Clone, Copy)]
enum Definition<'b> {
    Shape(&'b Shape),
    Morph(&'b MorphShape),
    Sprite(&'b Sprite),
    Other(u16),
}

/// Where the shapes of a timeline go: drawn, or into a mask.
#[derive(Clone, Copy)]
enum Target {
    Items,
    Mask(usize),
}

/// What a timeline inherits from the placement that shows it.
#[derive(Clone, Copy)]
struct Context {
    transform: Affine,
    color_transform: ColorTransform,
    mask: Option<usize>,
    target: Target,
    sprite: Option<u16>,
    depth: u32,
}

/// Composes the first frame of a movie into a [`Scene`].
pub(crate) struct Composer<'b> {
    definitions: HashMap<u16, Definition<'b>>,
    shapes: HashMap<(u16, u16), usize>,
    scene: Scene,
    placements: usize,
    paths: usize,
    stopped: bool,
}

impl<'b> Composer<'b> {
    /// Composes the display list of the main timeline's first frame.
    pub(crate) fn compose(assets: &'b Assets<'_>, stage: Rect) -> Scene {
        let mut definitions = HashMap::new();
        for other in &assets.other_definitions {
            definitions
                .entry(other.id)
                .or_insert(Definition::Other(other.code));
        }
        for font in &assets.fonts {
            definitions
                .entry(font.id)
                .or_insert(Definition::Other(TAG_DEFINE_FONT));
        }
        for bitmap in &assets.bitmaps {
            definitions
                .entry(bitmap.id)
                .or_insert(Definition::Other(TAG_DEFINE_BITS));
        }
        for sprite in &assets.sprites {
            definitions
                .entry(sprite.id)
                .or_insert(Definition::Sprite(sprite));
        }
        for morph in &assets.morph_shapes {
            definitions
                .entry(morph.id)
                .or_insert(Definition::Morph(morph));
        }
        for shape in &assets.shapes {
            definitions
                .entry(shape.id)
                .or_insert(Definition::Shape(shape));
        }
        let mut composer = Self {
            definitions,
            shapes: HashMap::new(),
            scene: Scene {
                stage,
                background: assets.background,
                ..Scene::default()
            },
            placements: 0,
            paths: 0,
            stopped: false,
        };
        let root = Context {
            transform: Affine::IDENTITY,
            color_transform: ColorTransform::IDENTITY,
            mask: None,
            target: Target::Items,
            sprite: None,
            depth: 0,
        };
        composer.timeline(&assets.timeline.first_frame, root);
        composer.scene
    }

    fn skip(&mut self, placement: &Placement, context: Context, reason: SkipReason) {
        self.scene.skipped.push(Skipped {
            id: placement.id,
            depth: placement.depth,
            sprite: context.sprite,
            reason,
        });
    }

    fn timeline(&mut self, placements: &[Placement], context: Context) {
        let mut masks: Vec<(usize, u16)> = Vec::new();
        for placement in placements {
            if self.stopped {
                return;
            }
            self.placements += 1;
            if self.placements > MAX_SCENE_PLACEMENTS {
                self.stop(placement, context, SkipReason::TooManyPlacements);
                return;
            }
            masks.retain(|&(_, clip)| clip >= placement.depth);
            if !placement.visible {
                continue;
            }
            let mask = masks.last().map(|&(mask, _)| mask).or(context.mask);
            if placement.has_filters {
                self.skip(placement, context, SkipReason::Filters);
            }
            if placement.blend_mode > 1 {
                let mode = placement.blend_mode;
                self.skip(placement, context, SkipReason::BlendMode { mode });
            }
            let mut inner = Context {
                transform: context.transform.then(&placement.matrix.into()),
                color_transform: context.color_transform.compose(&placement.color_transform),
                mask,
                ..context
            };
            if let Some(clip) = placement.clip_depth {
                if matches!(context.target, Target::Mask(_)) {
                    self.skip(placement, context, SkipReason::NestedMask);
                    continue;
                }
                if self.scene.masks.len() >= MAX_SCENE_MASKS {
                    self.skip(placement, context, SkipReason::TooManyMasks);
                    continue;
                }
                let index = self.scene.masks.len();
                self.scene.masks.push(SceneMask {
                    shapes: Vec::new(),
                    parent: mask,
                });
                masks.push((index, clip));
                inner.target = Target::Mask(index);
            }
            self.place(placement, context, inner);
        }
    }

    fn place(&mut self, placement: &Placement, context: Context, inner: Context) {
        match self.definitions.get(&placement.id).copied() {
            Some(Definition::Shape(shape)) => {
                let index = self.shape(shape.id, 0, |id| {
                    ShapeBuilder::convert(
                        id,
                        &shape.fill_styles,
                        &shape.line_styles,
                        &shape.records,
                    )
                });
                self.draw(placement, context, inner, index);
            }
            Some(Definition::Morph(morph)) => {
                let index = self.shape(morph.id, placement.ratio, |id| {
                    let (fills, lines, records) = morph_at(morph, placement.ratio);
                    ShapeBuilder::convert(id, &fills, &lines, &records)
                });
                self.draw(placement, context, inner, index);
            }
            Some(Definition::Sprite(sprite)) => {
                if inner.depth >= MAX_SPRITE_DEPTH {
                    self.skip(placement, context, SkipReason::SpriteDepth);
                    return;
                }
                let nested = Context {
                    sprite: Some(sprite.id),
                    depth: inner.depth + 1,
                    ..inner
                };
                self.timeline(&sprite.timeline.first_frame, nested);
            }
            Some(Definition::Other(code)) => {
                self.skip(placement, context, SkipReason::NotDrawn { code });
            }
            None => self.skip(placement, context, SkipReason::UnknownDefinition),
        }
    }

    fn shape(
        &mut self,
        id: u16,
        ratio: u16,
        convert: impl FnOnce(u16) -> (SceneShape, Vec<SkipReason>),
    ) -> (usize, Vec<SkipReason>) {
        if let Some(&index) = self.shapes.get(&(id, ratio)) {
            return (index, Vec::new());
        }
        let (shape, issues) = convert(id);
        let index = self.scene.shapes.len();
        self.scene.shapes.push(shape);
        self.shapes.insert((id, ratio), index);
        (index, issues)
    }

    fn draw(
        &mut self,
        placement: &Placement,
        context: Context,
        inner: Context,
        (index, issues): (usize, Vec<SkipReason>),
    ) {
        for issue in issues {
            self.skip(placement, context, issue);
        }
        let layers = self
            .scene
            .shapes
            .get(index)
            .map_or(0, |shape| shape.layers.len());
        self.paths = self.paths.saturating_add(layers);
        if self.paths > MAX_SCENE_PATHS {
            self.stop(placement, context, SkipReason::TooManyPaths);
            return;
        }
        if !inner.transform.is_finite() {
            self.skip(placement, context, SkipReason::InvalidGeometry);
            return;
        }
        match inner.target {
            Target::Items => self.scene.items.push(SceneItem {
                shape: index,
                transform: inner.transform,
                color_transform: inner.color_transform,
                mask: inner.mask,
                origin: Origin {
                    id: placement.id,
                    depth: placement.depth,
                    sprite: context.sprite,
                },
            }),
            Target::Mask(mask) => {
                if let Some(mask) = self.scene.masks.get_mut(mask) {
                    mask.shapes.push((index, inner.transform));
                }
            }
        }
    }

    fn stop(&mut self, placement: &Placement, context: Context, reason: SkipReason) {
        self.stopped = true;
        self.skip(placement, context, reason);
    }
}
