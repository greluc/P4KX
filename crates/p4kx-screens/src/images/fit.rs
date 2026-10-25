// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::{Affine, LayerKind, PathCommand, ScenePath, ScenePoint, ShapeLayer};

use crate::paint::{Area, TWIPS};
use crate::picture::{Fit, NineSlice, VectorFit};

/// Where a texture lies: the area it is clipped to and the area its whole image covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextureRect {
    pub(crate) clip: Area,
    pub(crate) image: Area,
}

fn offset(free: f32, place: f32) -> f32 {
    free * place.clamp(0.0, 1.0)
}

/// Fits an image of `size` pixels into `area` the way the scaling method says.
pub(crate) fn texture_rect(area: Area, size: (u32, u32), fit: Fit, place: [f32; 2]) -> TextureRect {
    #[expect(
        clippy::cast_precision_loss,
        reason = "texture sides are far below 2^24"
    )]
    let (width, height) = (size.0 as f32, size.1 as f32);
    let scale = match fit {
        Fit::Fill => {
            return TextureRect {
                clip: area,
                image: area,
            };
        }
        Fit::Contain => (area.width / width).min(area.height / height),
        Fit::Cover => (area.width / width).max(area.height / height),
    };
    let (shown_width, shown_height) = (width * scale, height * scale);
    let image = Area {
        x: area.x + offset(area.width - shown_width, place[0]),
        y: area.y + offset(area.height - shown_height, place[1]),
        width: shown_width,
        height: shown_height,
    };
    TextureRect {
        clip: if fit == Fit::Cover { area } else { image },
        image,
    }
}

/// The scale and the box of a vector graphic of `view` size fitted into `area`.
fn vector_box(area: Area, view: [f32; 4], fit: VectorFit, place: [f32; 2]) -> ([f32; 2], Area) {
    let [_, _, width, height] = view;
    let (scale_x, scale_y) = match fit {
        VectorFit::Contain => {
            let scale = (area.width / width).min(area.height / height);
            (scale, scale)
        }
        VectorFit::StretchXY => (area.width / width, area.height / height),
        VectorFit::StretchX => {
            let scale = area.width / width;
            (scale, scale.min(area.height / height))
        }
        VectorFit::StretchY => {
            let scale = area.height / height;
            (scale.min(area.width / width), scale)
        }
        VectorFit::Center => (1.0, 1.0),
    };
    let (shown_width, shown_height) = (width * scale_x, height * scale_y);
    (
        [scale_x, scale_y],
        Area {
            x: area.x + offset(area.width - shown_width, place[0]),
            y: area.y + offset(area.height - shown_height, place[1]),
            width: shown_width,
            height: shown_height,
        },
    )
}

fn flipped(flip: bool, start: f32, length: f32, at: f32, scale: f32) -> (f32, f32) {
    if flip {
        (-scale, at.mul_add(scale, start + length))
    } else {
        (scale, (-at).mul_add(scale, start))
    }
}

/// Maps user units of a vector graphic to the stage in twips.
pub(crate) fn vector_transform(
    area: Area,
    view: [f32; 4],
    fit: VectorFit,
    place: [f32; 2],
    flip: [bool; 2],
) -> Affine {
    let (scale, shown) = vector_box(area, view, fit, place);
    let (a, tx) = flipped(flip[0], shown.x, shown.width, view[0], scale[0]);
    let (d, ty) = flipped(flip[1], shown.y, shown.height, view[1], scale[1]);
    Affine {
        a: a * TWIPS,
        b: 0.0,
        c: 0.0,
        d: d * TWIPS,
        tx: tx * TWIPS,
        ty: ty * TWIPS,
    }
}

fn stretch(value: f32, length: f32, low: f32, high: f32, target: f32, scale: f32) -> f32 {
    let (low_at, high_at) = (low * length, high * length);
    let (mut low_cap, mut high_cap) = (low_at * scale, (length - high_at) * scale);
    if low_cap + high_cap > target {
        let shrink = target / (low_cap + high_cap);
        low_cap *= shrink;
        high_cap *= shrink;
    }
    if value <= low_at {
        if low_at > 0.0 {
            value * low_cap / low_at
        } else {
            0.0
        }
    } else if value >= high_at {
        let tail = length - high_at;
        let along = if tail > 0.0 {
            (value - high_at) * high_cap / tail
        } else {
            0.0
        };
        target - high_cap + along
    } else {
        low_cap + (value - low_at) * (target - low_cap - high_cap) / (high_at - low_at)
    }
}

fn sliced(low: f32, high: f32) -> bool {
    (0.0..=1.0).contains(&low) && (0.0..=1.0).contains(&high) && low < high
}

fn remap(
    layers: &[ShapeLayer],
    map: &dyn Fn(ScenePoint) -> ScenePoint,
    stroke_scale: f32,
) -> Vec<ShapeLayer> {
    layers
        .iter()
        .map(|layer| ShapeLayer {
            path: ScenePath {
                commands: layer
                    .path
                    .commands
                    .iter()
                    .map(|command| match *command {
                        PathCommand::MoveTo(p) => PathCommand::MoveTo(map(p)),
                        PathCommand::LineTo(p) => PathCommand::LineTo(map(p)),
                        PathCommand::QuadTo(c, p) => PathCommand::QuadTo(map(c), map(p)),
                        PathCommand::Close => PathCommand::Close,
                    })
                    .collect(),
            },
            paint: layer.paint.clone(),
            kind: match layer.kind {
                LayerKind::Fill => LayerKind::Fill,
                LayerKind::Stroke(mut style) => {
                    style.width *= stroke_scale;
                    LayerKind::Stroke(style)
                }
            },
        })
        .collect()
}

/// One axis of a nine-slice: where the graphic starts, how long it is shown, and how a
/// coordinate of the graphic maps onto it.
struct Axis {
    origin: f32,
    target: f32,
    start: f32,
    length: f32,
    cuts: Option<(f32, f32)>,
    scale: f32,
}

impl Axis {
    fn map(&self, value: f32, cap_scale: f32) -> f32 {
        let along = value - self.start;
        match self.cuts {
            Some((low, high)) => stretch(along, self.length, low, high, self.target, cap_scale),
            None => along * self.scale,
        }
    }
}

/// The layers of a vector graphic stretched with a nine-slice, in pixels from `origin`.
pub(crate) struct Sliced {
    pub(crate) layers: Vec<ShapeLayer>,
    pub(crate) origin: [f32; 2],
}

/// Stretches the layers of a vector graphic of `view` size over `area` with the caps of the
/// nine-slice kept at their size. An axis whose cut lines do not lie in order inside the graphic
/// is not sliced but fitted as `fit` says; `None` when neither axis is sliced.
pub(crate) fn vector_layers(
    layers: &[ShapeLayer],
    view: [f32; 4],
    slice: &NineSlice,
    area: Area,
    (fit, place): (VectorFit, [f32; 2]),
    flip: [bool; 2],
) -> Option<Sliced> {
    if !slice.scale.is_finite() || slice.scale <= 0.0 {
        return None;
    }
    let (scale, shown) = vector_box(area, view, fit, place);
    let x_cuts = sliced(slice.left, slice.right).then_some((slice.left, slice.right));
    let y_cuts = sliced(slice.top, slice.bottom).then_some((slice.top, slice.bottom));
    if x_cuts.is_none() && y_cuts.is_none() {
        return None;
    }
    let x = Axis {
        origin: if x_cuts.is_some() { area.x } else { shown.x },
        target: if x_cuts.is_some() {
            area.width
        } else {
            shown.width
        },
        start: view[0],
        length: view[2],
        cuts: x_cuts,
        scale: scale[0],
    };
    let y = Axis {
        origin: if y_cuts.is_some() { area.y } else { shown.y },
        target: if y_cuts.is_some() {
            area.height
        } else {
            shown.height
        },
        start: view[1],
        length: view[3],
        cuts: y_cuts,
        scale: scale[1],
    };
    let map = |point: ScenePoint| {
        let (px, py) = (x.map(point.x, slice.scale), y.map(point.y, slice.scale));
        ScenePoint {
            x: if flip[0] { x.target - px } else { px },
            y: if flip[1] { y.target - py } else { py },
        }
    };
    let layers = remap(layers, &map, slice.scale);
    Some(Sliced {
        layers,
        origin: [x.origin, y.origin],
    })
}
