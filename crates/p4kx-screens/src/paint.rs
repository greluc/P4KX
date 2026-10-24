// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_gfx::{
    Cap, ColorTransform, Join, LayerKind, Origin, Paint, PathCommand, Rect, Rgba, Scene, SceneItem,
    ScenePath, ScenePoint, SceneShape, ShapeLayer, StrokeStyle,
};

use crate::glyphs::GlyphKey;
use crate::paint_state::{PaintState, bounds_of};

pub(crate) const TWIPS: f32 = 20.0;
const ARC_SEGMENTS: u8 = 16;
const MITER_LIMIT: f32 = 4.0;

pub(crate) const MAX_SHAPES: usize = 1 << 17;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Area {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

const MAX_COORDINATE: f32 = 1.0e6;

impl Area {
    pub(crate) fn is_empty(&self) -> bool {
        let sane = [self.x, self.y, self.width, self.height]
            .iter()
            .all(|value| is_sane(*value));
        !(sane && self.width > 0.0 && self.height > 0.0)
    }

    pub(crate) fn inset(&self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            x: self.x + left,
            y: self.y + top,
            width: (self.width - left - right).max(0.0),
            height: (self.height - top - bottom).max(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Fill,
    Stroke(f32),
}

#[derive(Debug, Default)]
pub(crate) struct Painter {
    pub(crate) shapes: Vec<SceneShape>,
    pub(crate) items: Vec<SceneItem>,
    pub(crate) glyphs: HashMap<GlyphKey, Option<usize>>,
    pub(crate) state: PaintState,
}

impl Painter {
    pub(crate) const fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) const fn is_full(&self) -> bool {
        self.items.len() >= MAX_SHAPES
    }

    pub(crate) fn rect(&mut self, area: Area, colour: Rgba) {
        if !area.is_empty() {
            self.push(rectangle(area), colour, Mode::Fill);
        }
    }

    pub(crate) fn ellipse(&mut self, area: Area, colour: Rgba) {
        if !area.is_empty() {
            self.push(ellipse(area), colour, Mode::Fill);
        }
    }

    /// Fills the closed contours of `commands` with the even-odd rule.
    pub(crate) fn fill_outline(&mut self, commands: Vec<PathCommand>, colour: Rgba) {
        self.push(commands, colour, Mode::Fill);
    }

    pub(crate) fn stroke_ellipse(&mut self, area: Area, colour: Rgba, width: f32) {
        if !area.is_empty() {
            self.push(ellipse(area), colour, Mode::Stroke(width));
        }
    }

    pub(crate) fn polygon(&mut self, points: &[(f32, f32)], colour: Rgba) {
        if let Some(path) = polygon(points) {
            self.push(path, colour, Mode::Fill);
        }
    }

    pub(crate) fn stroke_polygon(&mut self, points: &[(f32, f32)], colour: Rgba, width: f32) {
        if let Some(path) = polygon(points) {
            self.push(path, colour, Mode::Stroke(width));
        }
    }

    fn push(&mut self, commands: Vec<PathCommand>, colour: Rgba, mode: Mode) {
        if colour.alpha == 0 || self.is_full() {
            return;
        }
        let (kind, reach) = match mode {
            Mode::Fill => (LayerKind::Fill, 0.0),
            Mode::Stroke(width) => (
                LayerKind::Stroke(StrokeStyle {
                    width: width * TWIPS,
                    cap: Cap::Butt,
                    join: Join::Miter,
                    miter_limit: MITER_LIMIT,
                }),
                width * TWIPS,
            ),
        };
        let path = ScenePath { commands };
        let transform = self.state.to_twips();
        let mask = self.mask_of(bounds_of(&path).as_ref(), reach, &transform);
        let index = self.shapes.len();
        let id = u16::try_from(index % usize::from(u16::MAX)).unwrap_or(0);
        self.shapes.push(SceneShape {
            id,
            layers: vec![ShapeLayer {
                path,
                paint: Paint::Solid(colour),
                kind,
            }],
        });
        self.items.push(SceneItem {
            shape: index,
            transform,
            color_transform: ColorTransform::IDENTITY,
            mask,
            origin: Origin {
                id,
                depth: 0,
                sprite: None,
            },
        });
    }

    pub(crate) fn finish(self, width: f32, height: f32) -> Scene {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the canvas side is checked to be at most 16384 pixels"
        )]
        let twips = |pixels: f32| (pixels * TWIPS).round() as i32;
        Scene {
            stage: Rect {
                x_min: 0,
                x_max: twips(width),
                y_min: 0,
                y_max: twips(height),
            },
            background: None,
            shapes: self.shapes,
            items: self.items,
            masks: self.state.masks,
            skipped: Vec::new(),
        }
    }
}

fn is_sane(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE
}

pub(crate) fn rectangle(area: Area) -> Vec<PathCommand> {
    let (left, top) = (area.x, area.y);
    let (right, bottom) = (area.x + area.width, area.y + area.height);
    vec![
        PathCommand::MoveTo(point(left, top)),
        PathCommand::LineTo(point(right, top)),
        PathCommand::LineTo(point(right, bottom)),
        PathCommand::LineTo(point(left, bottom)),
        PathCommand::Close,
    ]
}

fn ellipse(area: Area) -> Vec<PathCommand> {
    let (cx, cy) = (area.x + area.width / 2.0, area.y + area.height / 2.0);
    let (rx, ry) = (area.width / 2.0, area.height / 2.0);
    let step = std::f32::consts::TAU / f32::from(ARC_SEGMENTS);
    let reach = 1.0 / (step / 2.0).cos();
    let on = |angle: f32| point(rx.mul_add(angle.cos(), cx), ry.mul_add(angle.sin(), cy));
    let mut commands = vec![PathCommand::MoveTo(on(0.0))];
    for segment in 1..=ARC_SEGMENTS {
        let end = step * f32::from(segment);
        let middle = end - step / 2.0;
        commands.push(PathCommand::QuadTo(
            point(
                (rx * reach).mul_add(middle.cos(), cx),
                (ry * reach).mul_add(middle.sin(), cy),
            ),
            on(end),
        ));
    }
    commands.push(PathCommand::Close);
    commands
}

fn polygon(points: &[(f32, f32)]) -> Option<Vec<PathCommand>> {
    if points.len() < 3 || !points.iter().all(|(x, y)| is_sane(*x) && is_sane(*y)) {
        return None;
    }
    let mut commands: Vec<PathCommand> = points
        .iter()
        .enumerate()
        .map(|(index, (x, y))| {
            if index == 0 {
                PathCommand::MoveTo(point(*x, *y))
            } else {
                PathCommand::LineTo(point(*x, *y))
            }
        })
        .collect();
    commands.push(PathCommand::Close);
    Some(commands)
}

pub(crate) fn point(x: f32, y: f32) -> ScenePoint {
    ScenePoint {
        x: x * TWIPS,
        y: y * TWIPS,
    }
}
