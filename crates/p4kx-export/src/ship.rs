use p4kx_extraction::RecordTree;

use crate::contract::Ship;
use crate::guid_text::CatalogGuid;
use crate::playability_rules::{self, Evidence};
use crate::record_fields::{class_name, component, integer, locale_key, reference, text};
use crate::texts::Texts;

/// Returns the ship the Record tree `tree` describes, its texts resolved by `texts` and its
/// playability decided by `evidence`.
pub(crate) fn ship<'f>(tree: &RecordTree<'f>, texts: &Texts<'f>, evidence: &Evidence) -> Ship<'f> {
    let vehicle = component(tree, "VehicleComponentParams");
    Ship {
        guid: CatalogGuid(tree.guid()),
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
    }
}
