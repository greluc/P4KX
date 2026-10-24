// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::StructValue;

use crate::flex::Flex;
use crate::reader::{child, flag, float, text};
use crate::style::Modifiers;
use crate::widget::Widget;

const MAX_SCALE: f32 = 1.0e4;
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

fn finite_or(value: f32, fallback: f32, invalid: &mut bool) -> f32 {
    if value.is_finite() {
        value
    } else {
        *invalid = true;
        fallback
    }
}

fn pick<const N: usize>(base: [f32; N], over: [Option<f32>; N]) -> [f32; N] {
    let mut out = base;
    for (slot, value) in out.iter_mut().zip(over) {
        if let Some(value) = value {
            *slot = value;
        }
    }
    out
}

fn bound_with(bound: Option<Dim>, amount: Option<f32>) -> Option<Dim> {
    amount.map_or(bound, |amount| {
        Some(bound.unwrap_or(Dim::Fixed(0.0)).with_amount(amount))
    })
}

impl Geometry {
    pub(crate) fn of(widget: &Widget, style: &Modifiers) -> Self {
        let over = &style.geometry;
        let mut invalid = false;
        let size = [0, 1].map(|at| match over.size.get(at).copied().flatten() {
            Some(amount) => widget.size.get(at).copied().unwrap_or(Dim::Auto).with_amount(amount),
            None => widget.size.get(at).copied().unwrap_or(Dim::Auto),
        });
        let position = pick(widget.position, over.position);
        let shift = pick(widget.position_offset, over.position_offset);
        let turn = pick(widget.orientation, over.orientation);
        let pivot = pick(widget.pivot, over.pivot);
        let anchor = pick(widget.anchor, over.anchor);
        let scale = pick(widget.scale, over.scale);
        let mut padding = widget.padding;
        for (slot, value) in [
            &mut padding.top,
            &mut padding.right,
            &mut padding.bottom,
            &mut padding.left,
        ]
        .into_iter()
        .zip(over.padding)
        {
            if let Some(value) = value {
                *slot = value;
            }
        }
        let mut flex = widget.flex;
        if let Some(flex) = &mut flex {
            flex.apply_gaps(over.gap);
        }
        let bounds = Bounds {
            min: [0, 1].map(|at| {
                bound_with(
                    widget.bounds.min.get(at).copied().flatten(),
                    over.min.get(at).copied().flatten(),
                )
            }),
            max: [0, 1].map(|at| {
                bound_with(
                    widget.bounds.max.get(at).copied().flatten(),
                    over.max.get(at).copied().flatten(),
                )
            }),
        };
        let rotation = [0, 1, 2].map(|at| {
            let total = turn.get(at).copied().unwrap_or(0.0)
                + widget.orientation_offset.get(at).copied().unwrap_or(0.0);
            finite_or(total, 0.0, &mut invalid).clamp(-MAX_TURN, MAX_TURN)
        });
        Self {
            size,
            bounds,
            offset: [0, 1].map(|at| {
                position.get(at).copied().unwrap_or(0.0) + shift.get(at).copied().unwrap_or(0.0)
            }),
            padding,
            margin: widget.margin,
            pivot: pivot.map(|value| finite_or(value, 0.0, &mut invalid)),
            anchor: anchor.map(|value| finite_or(value, 0.0, &mut invalid)),
            rotation,
            scale: scale.map(|value| finite_or(value, 1.0, &mut invalid).clamp(-MAX_SCALE, MAX_SCALE)),
            flex,
            invalid,
        }
    }
}
