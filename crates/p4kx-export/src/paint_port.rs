// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{LoadoutFileError, VehicleImplementation};

use crate::guid_text::CatalogGuid;
use crate::variants::ShipKey;

const PAINTS: &str = "Paints";

/// An item port of paints, in a ship's Record or its vehicle implementation: the tags an item
/// needs to carry to fit it and the tags the port offers, as lower-case tag names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct PaintPort {
    pub(crate) port_tags: Vec<String>,
    pub(crate) required_tags: Vec<String>,
}

impl PaintPort {
    /// Returns the port with the white-space separated tag lists `port_tags` and `required_tags`.
    pub(crate) fn new(port_tags: Option<&str>, required_tags: Option<&str>) -> Self {
        Self {
            port_tags: tags(port_tags),
            required_tags: tags(required_tags),
        }
    }
}

fn tags(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect()
}

/// What the paints of a ship are derived from: the paint ports of its Record and of the vehicle
/// implementation the ship base data read.
#[derive(Debug)]
pub(crate) struct VehicleFacts {
    pub(crate) ship: CatalogGuid,
    pub(crate) paint_ports: Vec<PaintPort>,
    pub(crate) implementation: VehiclePorts,
    pub(crate) variant_key: Option<ShipKey>,
}

/// The paint ports of a vehicle implementation and whether it was not read in full. A patch file
/// the Archive lacks is a fact of the game data, not an unread file.
#[derive(Debug, Clone, Default)]
pub(crate) struct VehiclePorts {
    pub(crate) ports: Vec<PaintPort>,
    pub(crate) unreadable: bool,
}

impl VehiclePorts {
    /// Returns the paint ports of `vehicle`, the implementation read for a ship with the
    /// Modification applied: its item ports that take `Paints`, with the required tags the patch
    /// leaves them. `vehicle` is `None` when the ship names no implementation or it could not be
    /// read; `named` tells which of the two it is.
    pub(crate) fn of(vehicle: Option<&VehicleImplementation>, named: bool) -> Self {
        let Some(implementation) = vehicle else {
            return Self {
                ports: Vec::new(),
                unreadable: named,
            };
        };
        let missing = LoadoutFileError::NotFound.to_string();
        Self {
            ports: implementation
                .item_ports()
                .filter_map(|part| part.item_port())
                .filter(|port| {
                    port.types()
                        .iter()
                        .any(|kind| kind.eq_ignore_ascii_case(PAINTS))
                })
                .map(|port| PaintPort::new(Some(port.port_tags()), Some(port.required_tags())))
                .collect(),
            unreadable: implementation
                .issues()
                .iter()
                .any(|issue| !issue.to_string().ends_with(&missing)),
        }
    }
}
