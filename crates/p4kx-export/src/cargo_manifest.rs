// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, StructValue};

use crate::contract_cargo::{
    CargoFillType, CargoManifest, CargoManifestResource, CargoSpawnKind, CargoSpawnRule,
};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{
    boolean, class_name, integer, number, reference, references, structs, structure,
};

const EVEN: &str = "CargoFillCapacityValue_Even";
const RANDOM: &str = "CargoFillCapacityValue_Random";
const CUSTOM: &str = "CargoFillCapacityValue_Custom";
const ENTITY_RULE: &str = "CargoSpawnEntityDef";
const RESOURCE_RULE: &str = "CargoSpawnResourceDef";

/// Returns the cargo manifest the Record tree `tree` describes.
pub(crate) fn cargo_manifest<'f>(tree: &RecordTree<'f>) -> CargoManifest<'f> {
    let root = Some(tree.root());
    let fill = structure(root, "cargoFillCapacity");
    CargoManifest {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        fill_type: fill.and_then(|fill| match fill.type_name() {
            EVEN => Some(CargoFillType::Even),
            RANDOM => Some(CargoFillType::Random),
            CUSTOM => Some(CargoFillType::Custom),
            _ => None,
        }),
        min_capacity_range: number(fill, "minCapacityRange"),
        max_capacity_range: number(fill, "maxCapacityRange"),
        lock_cargo_grids_on_spawn: boolean(root, "lockCargoGridsOnSpawn"),
        resources: structs(fill, "resources").map(resource).collect(),
        spawn_rules: structs(root, "cargoSpawnRules")
            .filter_map(spawn_rule)
            .collect(),
    }
}

/// A `CargoResource`, or a `CargoResourceAllocation` that holds one in `resources`.
fn resource(entry: &StructValue<'_>) -> CargoManifestResource {
    let inner = structure(Some(entry), "resources");
    let cargo = inner.unwrap_or(entry);
    CargoManifestResource {
        resource_guid: reference(Some(cargo), "resource"),
        probability: number(Some(cargo), "probability"),
        min_allocation: number(Some(entry), "minResourceAllocationPercentage"),
        max_allocation: number(Some(entry), "maxResourceAllocationPercentage"),
    }
}

fn spawn_rule<'f>(rule: &StructValue<'f>) -> Option<CargoSpawnRule<'f>> {
    let kind = match rule.type_name() {
        ENTITY_RULE => CargoSpawnKind::Entities,
        RESOURCE_RULE => CargoSpawnKind::Resources,
        _ => return None,
    };
    let algorithm = structure(Some(rule), "quantityAlgorithmType");
    Some(CargoSpawnRule {
        kind,
        probability: number(Some(rule), "probability"),
        entity_guids: references(Some(rule), "entities"),
        resource_guids: references(Some(rule), "resources"),
        quantity_algorithm: algorithm.map(StructValue::type_name),
        quantity: integer(algorithm, "quantity"),
        proportion: number(algorithm, "proportion"),
    })
}
