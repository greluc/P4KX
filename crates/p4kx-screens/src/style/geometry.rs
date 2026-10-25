// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::StructValue;

use super::flex::FlexModifiers;
use crate::reader::text;

const ENUM_PREFIX: &str = "BuildingBlocks_FieldModifierEnumeratedType";

/// How a size modifier reads its amount: the `WidthBehavior` and `HeightBehavior` of a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Behaviour {
    Fixed,
    Percent,
    OfWidth,
    OfHeight,
    Auto,
}

impl Behaviour {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "Fixed" => Some(Self::Fixed),
            "Percent" => Some(Self::Percent),
            "PercentOfX" => Some(Self::OfWidth),
            "PercentOfY" => Some(Self::OfHeight),
            "Auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// The geometry fields a style entry overrides: size and how it is read, bounds, anchor, pivot,
/// scale, orientation, position, padding, margin, the growth of flex items and the settings and
/// spacing of flex containers.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct GeometryModifiers {
    pub(crate) size: [Option<f32>; 2],
    pub(crate) behaviour: [Option<Behaviour>; 2],
    pub(crate) min: [Option<f32>; 2],
    pub(crate) max: [Option<f32>; 2],
    pub(crate) anchor: [Option<f32>; 2],
    pub(crate) pivot: [Option<f32>; 2],
    pub(crate) scale: [Option<f32>; 2],
    pub(crate) orientation: [Option<f32>; 3],
    pub(crate) orientation_offset: [Option<f32>; 3],
    pub(crate) position: [Option<f32>; 2],
    pub(crate) position_offset: [Option<f32>; 2],
    pub(crate) padding: [Option<f32>; 4],
    pub(crate) margin: [Option<f32>; 4],
    pub(crate) gap: [Option<f32>; 2],
    pub(crate) item: [Option<f32>; 2],
    pub(crate) flex: FlexModifiers,
    pub(crate) radius: [Option<f32>; 4],
    pub(crate) chamfer: [Option<bool>; 4],
}

const CORNERS: [&str; 4] = ["TopLeft", "TopRight", "BottomRight", "BottomLeft"];

impl GeometryModifiers {
    /// Takes the boolean field `field` when it enables a chamfer, `false` otherwise.
    pub(crate) fn flag(&mut self, field: &str, value: Option<bool>) -> bool {
        let corner = field
            .strip_prefix("Enable")
            .and_then(|rest| rest.strip_suffix("BorderChamfer"));
        let slot = corner
            .and_then(|corner| CORNERS.iter().position(|name| *name == corner))
            .and_then(|at| self.chamfer.get_mut(at));
        slot.is_some_and(|slot| {
            *slot = value;
            true
        })
    }

    /// Takes the enumerated modifier `inner` when it sets how a size is read or a setting of a
    /// flex container.
    pub(crate) fn typed(&mut self, inner: &StructValue<'_>) {
        let Some(kind) = inner.type_name().strip_prefix(ENUM_PREFIX) else {
            return;
        };
        let Some(value) = text(inner, "value") else {
            return;
        };
        match kind {
            "WidthBehavior" => self.behaviour[0] = Behaviour::parse(value),
            "HeightBehavior" => self.behaviour[1] = Behaviour::parse(value),
            other => self.flex.typed(other, value),
        }
    }

    /// Takes the numeric field `field` when it is one of the geometry, `false` otherwise.
    pub(crate) fn number(&mut self, field: &str, value: Option<f32>) -> bool {
        let table: [(&str, &mut [Option<f32>]); 9] = [
            ("Size", &mut self.size),
            ("MinSize", &mut self.min),
            ("MaxSize", &mut self.max),
            ("Anchor", &mut self.anchor),
            ("Pivot", &mut self.pivot),
            ("Scale", &mut self.scale),
            ("Orientation", &mut self.orientation),
            ("Padding", &mut self.padding),
            ("Margin", &mut self.margin),
        ];
        for (prefix, slots) in table {
            if let Some(rest) = field.strip_prefix(prefix)
                && let Some(at) = suffix_index(prefix, rest)
                && let Some(slot) = slots.get_mut(at)
            {
                *slot = value;
                return true;
            }
        }
        self.positional(field, value)
    }

    fn positional(&mut self, field: &str, value: Option<f32>) -> bool {
        let corner = field
            .strip_prefix("Border")
            .and_then(|rest| rest.strip_suffix("Radius"))
            .and_then(|corner| CORNERS.iter().position(|name| *name == corner));
        if let Some(slot) = corner.and_then(|at| self.radius.get_mut(at)) {
            *slot = value;
            return true;
        }
        let slot = match field {
            "PosX" => self.position.get_mut(0),
            "PosY" => self.position.get_mut(1),
            "PosXOffset" => self.position_offset.get_mut(0),
            "PosYOffset" => self.position_offset.get_mut(1),
            "OrientationXOffset" => self.orientation_offset.get_mut(0),
            "OrientationYOffset" => self.orientation_offset.get_mut(1),
            "OrientationZOffset" => self.orientation_offset.get_mut(2),
            "FlexColumnSpacing" => self.gap.get_mut(0),
            "FlexRowSpacing" => self.gap.get_mut(1),
            "FlexGrowProportion" => self.item.get_mut(0),
            "FlexShrinkProportion" => self.item.get_mut(1),
            _ => None,
        };
        slot.is_some_and(|slot| {
            *slot = value;
            true
        })
    }

    /// Lets the fields `other` sets replace those of `self`.
    pub(crate) fn merge(&mut self, other: &Self) {
        fn lay<T: Copy>(into: &mut [Option<T>], from: &[Option<T>]) {
            for (slot, value) in into.iter_mut().zip(from) {
                if value.is_some() {
                    *slot = *value;
                }
            }
        }
        lay(&mut self.size, &other.size);
        lay(&mut self.behaviour, &other.behaviour);
        lay(&mut self.min, &other.min);
        lay(&mut self.max, &other.max);
        lay(&mut self.anchor, &other.anchor);
        lay(&mut self.pivot, &other.pivot);
        lay(&mut self.scale, &other.scale);
        lay(&mut self.orientation, &other.orientation);
        lay(&mut self.orientation_offset, &other.orientation_offset);
        lay(&mut self.position, &other.position);
        lay(&mut self.position_offset, &other.position_offset);
        lay(&mut self.padding, &other.padding);
        lay(&mut self.margin, &other.margin);
        lay(&mut self.gap, &other.gap);
        lay(&mut self.item, &other.item);
        lay(&mut self.radius, &other.radius);
        lay(&mut self.chamfer, &other.chamfer);
        self.flex.merge(other.flex);
    }
}

fn suffix_index(prefix: &str, rest: &str) -> Option<usize> {
    if matches!(prefix, "Padding" | "Margin") {
        return ["Top", "Right", "Bottom", "Left"]
            .iter()
            .position(|side| *side == rest);
    }
    ["X", "Y", "Z"].iter().position(|axis| *axis == rest)
}
