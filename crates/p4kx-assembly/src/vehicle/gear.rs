// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_dataforge::{RecordTree, Value};

use super::LandingGear;
use crate::read::{as_struct, float, struct_field, text};

/// Reads the gears of a `VehicleLandingGearSystem` Record.
pub(crate) fn gears(tree: &RecordTree<'_>) -> Arc<[LandingGear]> {
    let items = match tree.field("gears") {
        Some(Value::Array(items)) => &items[..],
        _ => &[],
    };
    items
        .iter()
        .filter_map(as_struct)
        .map(|gear| LandingGear {
            bone: text(gear, "bone").to_owned(),
            geometry: struct_field(gear, "geometry")
                .map_or("", |geometry| text(geometry, "path"))
                .to_owned(),
            mass: float(gear, "mass"),
        })
        .collect()
}
