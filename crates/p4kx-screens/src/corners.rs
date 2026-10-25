// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::f32::consts::FRAC_PI_2;

use p4kx_gfx::PathCommand;

use crate::geometry::Dim;
use crate::paint::{Area, point};

const ARC_STEPS: u8 = 3;

/// The corner of a box as the Records set it: a radius, rounded or chamfered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Corner {
    pub(crate) radius: Dim,
    pub(crate) chamfer: bool,
}

impl Corner {
    pub(crate) const SQUARE: Self = Self {
        radius: Dim::Fixed(0.0),
        chamfer: false,
    };
}

/// The four corners of a box, from the top left clockwise, with their radii in layout units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Corners {
    radii: [f32; 4],
    chamfer: [bool; 4],
}

impl Corners {
    /// Resolves the corners of a box of `area`: a share of a radius is of the shorter side, and
    /// no radius passes half of it.
    pub(crate) fn of(specs: &[Corner; 4], area: Area) -> Self {
        let shorter = area.width.min(area.height);
        let half = shorter / 2.0;
        let radius = |corner: &Corner| {
            let value = match corner.radius {
                Dim::Fixed(value) => value,
                Dim::Percent(share) => share * shorter,
                Dim::OfWidth(share) => share * area.width,
                Dim::OfHeight(share) => share * area.height,
                Dim::Auto => 0.0,
            };
            if value.is_finite() {
                value.clamp(0.0, half.max(0.0))
            } else {
                0.0
            }
        };
        Self {
            radii: specs.each_ref().map(radius),
            chamfer: specs.map(|corner| corner.chamfer),
        }
    }

    pub(crate) fn is_square(&self) -> bool {
        self.radii.iter().all(|radius| *radius <= 0.0)
    }

    /// Returns the corners of the box inside borders of the widths `[top, right, bottom, left]`.
    pub(crate) fn inside(&self, [top, right, bottom, left]: [f32; 4]) -> Self {
        let shrink = |radius: f32, vertical: f32, horizontal: f32| {
            (radius - vertical.max(horizontal)).max(0.0)
        };
        let [first, second, third, fourth] = self.radii;
        Self {
            radii: [
                shrink(first, top, left),
                shrink(second, top, right),
                shrink(third, bottom, right),
                shrink(fourth, bottom, left),
            ],
            chamfer: self.chamfer,
        }
    }
}

/// Appends the corner of radius `radius` about `centre`, from the direction `start` a quarter
/// turn on, clockwise, as a bevel or as arcs of quadratic curves.
fn corner(
    commands: &mut Vec<PathCommand>,
    centre: (f32, f32),
    radius: f32,
    start: f32,
    bevel: bool,
) {
    let at = |angle: f32, reach: f32| {
        point(
            (radius * reach).mul_add(angle.cos(), centre.0),
            (radius * reach).mul_add(angle.sin(), centre.1),
        )
    };
    let step = FRAC_PI_2 / f32::from(ARC_STEPS);
    if bevel {
        commands.push(PathCommand::LineTo(at(start + FRAC_PI_2, 1.0)));
        return;
    }
    let reach = 1.0 / (step / 2.0).cos();
    for part in 1..=ARC_STEPS {
        let end = step.mul_add(f32::from(part), start);
        commands.push(PathCommand::QuadTo(
            at(end - step / 2.0, reach),
            at(end, 1.0),
        ));
    }
}

/// Returns the closed contour of the box `area` with its corners.
pub(crate) fn outline(area: Area, corners: &Corners) -> Vec<PathCommand> {
    let [first, second, third, fourth] = corners.radii;
    let [bevel_first, bevel_second, bevel_third, bevel_fourth] = corners.chamfer;
    let (left, top) = (area.x, area.y);
    let (right, bottom) = (area.x + area.width, area.y + area.height);
    let mut commands = vec![
        PathCommand::MoveTo(point(left + first, top)),
        PathCommand::LineTo(point(right - second, top)),
    ];
    corner(
        &mut commands,
        (right - second, top + second),
        second,
        -FRAC_PI_2,
        bevel_second,
    );
    commands.push(PathCommand::LineTo(point(right, bottom - third)));
    corner(
        &mut commands,
        (right - third, bottom - third),
        third,
        0.0,
        bevel_third,
    );
    commands.push(PathCommand::LineTo(point(left + fourth, bottom)));
    corner(
        &mut commands,
        (left + fourth, bottom - fourth),
        fourth,
        FRAC_PI_2,
        bevel_fourth,
    );
    commands.push(PathCommand::LineTo(point(left, top + first)));
    corner(
        &mut commands,
        (left + first, top + first),
        first,
        2.0 * FRAC_PI_2,
        bevel_first,
    );
    commands.push(PathCommand::Close);
    commands
}
