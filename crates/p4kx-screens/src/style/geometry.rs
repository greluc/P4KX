// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// The geometry fields a style entry overrides: size, bounds, anchor, pivot, scale, orientation,
/// position, padding and the spacing of flex containers.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct GeometryModifiers {
    pub(crate) size: [Option<f32>; 2],
    pub(crate) min: [Option<f32>; 2],
    pub(crate) max: [Option<f32>; 2],
    pub(crate) anchor: [Option<f32>; 2],
    pub(crate) pivot: [Option<f32>; 2],
    pub(crate) scale: [Option<f32>; 2],
    pub(crate) orientation: [Option<f32>; 3],
    pub(crate) position: [Option<f32>; 2],
    pub(crate) position_offset: [Option<f32>; 2],
    pub(crate) padding: [Option<f32>; 4],
    pub(crate) gap: [Option<f32>; 2],
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

    /// Takes the numeric field `field` when it is one of the geometry, `false` otherwise.
    pub(crate) fn number(&mut self, field: &str, value: Option<f32>) -> bool {
        let table: [(&str, &mut [Option<f32>]); 8] = [
            ("Size", &mut self.size),
            ("MinSize", &mut self.min),
            ("MaxSize", &mut self.max),
            ("Anchor", &mut self.anchor),
            ("Pivot", &mut self.pivot),
            ("Scale", &mut self.scale),
            ("Orientation", &mut self.orientation),
            ("Padding", &mut self.padding),
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
            "FlexColumnSpacing" => self.gap.get_mut(0),
            "FlexRowSpacing" => self.gap.get_mut(1),
            _ => None,
        };
        slot.is_some_and(|slot| {
            *slot = value;
            true
        })
    }

    /// Lets the fields `other` sets replace those of `self`.
    pub(crate) fn merge(&mut self, other: &Self) {
        fn lay(into: &mut [Option<f32>], from: &[Option<f32>]) {
            for (slot, value) in into.iter_mut().zip(from) {
                if value.is_some() {
                    *slot = *value;
                }
            }
        }
        lay(&mut self.size, &other.size);
        lay(&mut self.min, &other.min);
        lay(&mut self.max, &other.max);
        lay(&mut self.anchor, &other.anchor);
        lay(&mut self.pivot, &other.pivot);
        lay(&mut self.scale, &other.scale);
        lay(&mut self.orientation, &other.orientation);
        lay(&mut self.position, &other.position);
        lay(&mut self.position_offset, &other.position_offset);
        lay(&mut self.padding, &other.padding);
        lay(&mut self.gap, &other.gap);
        lay(&mut self.radius, &other.radius);
        for (slot, value) in self.chamfer.iter_mut().zip(other.chamfer) {
            if value.is_some() {
                *slot = value;
            }
        }
    }
}

fn suffix_index(prefix: &str, rest: &str) -> Option<usize> {
    if prefix == "Padding" {
        return ["Top", "Right", "Bottom", "Left"]
            .iter()
            .position(|side| *side == rest);
    }
    ["X", "Y", "Z"].iter().position(|axis| *axis == rest)
}
