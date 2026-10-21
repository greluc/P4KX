use p4kx_extraction::{RecordTree, StructValue, Value};

use crate::contract::Ship;
use crate::guid_text::CatalogGuid;
use crate::paint_port::{PaintPort, VehicleFacts};
use crate::playability_rules::{self, Evidence};
use crate::record_fields::{class_name, component, elements, integer, locale_key, reference, text};
use crate::texts::Texts;

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
/// playability decided by `evidence`, with the vehicle facts its paints are derived from.
pub(crate) fn ship<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    evidence: &Evidence,
) -> (Ship<'f>, VehicleFacts<'f>) {
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
    };
    let facts = VehicleFacts {
        ship: guid,
        definition: ship.vehicle_definition,
        modification: text(vehicle, "modification"),
        paint_ports: paint_ports(tree),
    };
    (ship, facts)
}
