// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::StructValue;

use crate::flex::Flex;
use crate::reader::{child, flag, float, text};
use crate::style::Modifiers;
use crate::widget::Widget;

const MAX_SCALE: f32 = 1.0e4;
const MAX_SHARE: f32 = 1.0e4;
const MAX_STYLE: f32 = 1.0e6;
const MAX_TURN: f32 = 1.0e6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Dim {
    Fixed(f32),
    Percent(f32),
    OfWidth(f32),
    OfHeight(f32),
    Auto,
}

impl Dim {
    pub(crate) fn parse(value: Option<&StructValue<'_>>) -> Self {
        let Some(value) = value else {
            return Self::Auto;
        };
        let amount = float(value, "value").unwrap_or(0.0);
        match text(value, "behavior") {
            Some("Percent") => Self::Percent(amount),
            Some("PercentOfX") => Self::OfWidth(amount),
            Some("PercentOfY") => Self::OfHeight(amount),
            Some("Auto") => Self::Auto,
            _ => Self::Fixed(amount),
        }
    }

    pub(crate) const fn with_amount(self, amount: f32) -> Self {
        match self {
            Self::Fixed(_) | Self::Auto => Self::Fixed(amount),
            Self::Percent(_) => Self::Percent(amount),
            Self::OfWidth(_) => Self::OfWidth(amount),
            Self::OfHeight(_) => Self::OfHeight(amount),
        }
    }

    pub(crate) fn along(self, parent: f32, other: f32) -> Option<f32> {
        match self {
            Self::Fixed(value) => Some(value),
            Self::Percent(value) => Some(value * parent),
            Self::OfWidth(value) | Self::OfHeight(value) => Some(value * other),
            Self::Auto => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Edges {
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
    pub(crate) left: f32,
}

impl Edges {
    pub(crate) fn parse(value: Option<&StructValue<'_>>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };
        let edge = |name| float(value, name).unwrap_or(0.0);
        Self {
            top: edge("top"),
            right: edge("right"),
            bottom: edge("bottom"),
            left: edge("left"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Overflow {
    #[default]
    Visible,
    Clip,
    ClipFade,
    Fade,
}

impl Overflow {
    pub(crate) fn parse(structure: &StructValue<'_>) -> Self {
        match child(structure, "overflow").and_then(|overflow| text(overflow, "overflow")) {
            Some("Clip") => Self::Clip,
            Some("ClipFade") => Self::ClipFade,
            Some("Fade") => Self::Fade,
            _ => Self::Visible,
        }
    }

    pub(crate) const fn clips(self) -> bool {
        matches!(self, Self::Clip | Self::ClipFade)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) min: [Option<Dim>; 2],
    pub(crate) max: [Option<Dim>; 2],
}

impl Bounds {
    pub(crate) fn parse(sizing: Option<&StructValue<'_>>) -> Self {
        let Some(sizing) = sizing else {
            return Self::default();
        };
        let bound = |enable, name| {
            flag(sizing, enable)
                .unwrap_or(false)
                .then(|| Dim::parse(child(sizing, name)))
        };
        Self {
            min: [
                bound("enableMinWidth", "minWidth"),
                bound("enableMinHeight", "minHeight"),
            ],
            max: [
                bound("enableMaxWidth", "maxWidth"),
                bound("enableMaxHeight", "maxHeight"),
            ],
        }
    }
}

/// The geometry of a widget after the style cascade: sizes, offsets, spacing, the transform
/// parameters and the flex container.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geometry {
    pub(crate) size: [Dim; 2],
    pub(crate) bounds: Bounds,
    pub(crate) offset: [f32; 2],
    pub(crate) padding: Edges,
    pub(crate) margin: Edges,
    pub(crate) pivot: [f32; 2],
    pub(crate) anchor: [f32; 2],
    pub(crate) rotation: [f32; 3],
    pub(crate) scale: [f32; 2],
    pub(crate) flex: Option<Flex>,
    pub(crate) invalid: bool,
}

const fn finite_or(value: f32, fallback: f32, invalid: &mut bool) -> f32 {
    if value.is_finite() {
        value
    } else {
        *invalid = true;
        fallback
    }
}

/// Returns a number a style entry sets when it is finite, held to a sane range.
fn limited(value: Option<f32>) -> Option<f32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(-MAX_STYLE, MAX_STYLE))
}

fn pick<const N: usize>(base: [f32; N], over: [Option<f32>; N]) -> [f32; N] {
    let mut out = base;
    for (slot, value) in out.iter_mut().zip(over) {
        if let Some(value) = limited(value) {
            *slot = value;
        }
    }
    out
}

fn bound_with(bound: Option<Dim>, amount: Option<f32>) -> Option<Dim> {
    limited(amount).map_or(bound, |amount| {
        Some(bound.unwrap_or(Dim::Fixed(0.0)).with_amount(amount))
    })
}

fn sized(base: [Dim; 2], over: [Option<f32>; 2]) -> [Dim; 2] {
    let mut out = base;
    for (dim, amount) in out.iter_mut().zip(over) {
        if let Some(amount) = limited(amount) {
            *dim = dim.with_amount(amount);
        }
    }
    out
}

fn bounded(base: [Option<Dim>; 2], over: [Option<f32>; 2]) -> [Option<Dim>; 2] {
    let mut out = base;
    for (slot, amount) in out.iter_mut().zip(over) {
        *slot = bound_with(*slot, amount);
    }
    out
}

fn padded(base: Edges, [top, right, bottom, left]: [Option<f32>; 4]) -> Edges {
    Edges {
        top: limited(top).unwrap_or(base.top),
        right: limited(right).unwrap_or(base.right),
        bottom: limited(bottom).unwrap_or(base.bottom),
        left: limited(left).unwrap_or(base.left),
    }
}

fn sane<const N: usize>(
    values: [f32; N],
    fallback: f32,
    limit: f32,
    invalid: &mut bool,
) -> [f32; N] {
    values.map(|value| finite_or(value, fallback, invalid).clamp(-limit, limit))
}

impl Geometry {
    pub(crate) fn of(widget: &Widget, style: &Modifiers) -> Self {
        let (over, pose) = (&style.geometry, &widget.pose);
        let mut invalid = false;
        let position = pick(pose.position, over.position);
        let shift = pick(pose.position_offset, over.position_offset);
        let mut turn = pick(pose.orientation, over.orientation);
        for (angle, extra) in turn.iter_mut().zip(pose.orientation_offset) {
            *angle += extra;
        }
        Self {
            size: sized(widget.size, over.size),
            bounds: Bounds {
                min: bounded(widget.bounds.min, over.min),
                max: bounded(widget.bounds.max, over.max),
            },
            offset: [position[0] + shift[0], position[1] + shift[1]],
            padding: padded(widget.padding, over.padding),
            margin: widget.margin,
            pivot: sane(pick(pose.pivot, over.pivot), 0.0, MAX_SHARE, &mut invalid),
            anchor: sane(pick(pose.anchor, over.anchor), 0.0, MAX_SHARE, &mut invalid),
            rotation: sane(turn, 0.0, MAX_TURN, &mut invalid),
            scale: sane(pick(pose.scale, over.scale), 1.0, MAX_SCALE, &mut invalid),
            flex: widget.flex.map(|mut flex| {
                flex.apply_gaps(over.gap);
                flex
            }),
            invalid,
        }
    }
}
