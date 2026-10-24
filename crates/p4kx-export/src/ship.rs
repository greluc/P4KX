// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, StructValue, Value};

use crate::contract::Ship;
use crate::guid_text::CatalogGuid;
use crate::paint_port::{PaintPort, VehicleFacts};
use crate::playability_rules::{self, Evidence};
use crate::record_fields::{class_name, component, elements, integer, locale_key, reference, text};
use crate::ship_base::ShipRead;
use crate::texts::Texts;
use crate::variants::ShipKey;

const PAINTS: &str = "Paints";

/// Returns the item ports of the Record tree `tree` that take paints: the `SItemPortDef` entries
/// of its `SItemPortContainerComponentParams` with a type `Paints`.
fn paint_ports(tree: &RecordTree<'_>) -> Vec<PaintPort> {
    let container = component(tree, "SItemPortContainerComponentParams");
    elements(container, "Ports")
        .iter()
        .filter_map(|port| match port {
            Value::Struct(port) if takes_paints(port) => Some(PaintPort::new(
                text(Some(port), "PortTags"),
                text(Some(port), "RequiredPortTags"),
            )),
            _ => None,
        })
        .collect()
}

fn takes_paints(port: &StructValue<'_>) -> bool {
    elements(Some(port), "Types").iter().any(|kind| match kind {
        Value::Struct(kind) => {
            text(Some(kind), "Type").is_some_and(|name| name.eq_ignore_ascii_case(PAINTS))
        }
        _ => false,
    })
}

/// Returns the ship the Record tree `tree` describes, its texts resolved by `texts` and its
/// playability decided by `evidence` and its mass, hull, cargo, size and kind taken from `read`,
/// with the vehicle facts its paints and variant group are derived from.
pub(crate) fn ship<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    evidence: &Evidence,
    read: &ShipRead,
) -> (Ship<'f>, VehicleFacts<'f>) {
    let base = &read.base;
    let vehicle = component(tree, "VehicleComponentParams");
    let guid = CatalogGuid(tree.guid());
    let ship = Ship {
        guid,
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(locale_key(vehicle, "vehicleName")),
        description: texts.localised(locale_key(vehicle, "vehicleDescription")),
        role: texts.localised(locale_key(vehicle, "vehicleRole")),
        career: texts.localised(locale_key(vehicle, "vehicleCareer")),
        crew_size: integer(vehicle, "crewSize"),
        manufacturer_guid: reference(vehicle, "manufacturer"),
        vehicle_definition: text(vehicle, "vehicleDefinition"),
        playability: playability_rules::ship(tree, evidence),
        vehicle_kind: base.vehicle_kind,
        size: base.size.as_ref().map(|size| size.value),
        mass: base.mass.as_ref().map(|mass| mass.kg),
        mass_basis: base.mass.as_ref().map(|mass| mass.basis),
        hull_health: base.hull.as_ref().map(|hull| hull.health),
        cargo_scu: base.cargo.scu,
        cargo_grids_with_issue: base.cargo.grids_with_issue,
        variant_of: None,
    };
    let facts = VehicleFacts {
        ship: guid,
        class_name: ship.class_name,
        definition: ship.vehicle_definition,
        modification: text(vehicle, "modification"),
        paint_ports: paint_ports(tree),
        variant_key: ship
            .vehicle_definition
            .zip(read.implementation)
            .map(|(definition, implementation)| ShipKey::new(definition, implementation)),
    };
    (ship, facts)
}
