// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestModel, TestNode,
    entity_schema, landing_system_record,
};

/// The class name of the ship of [`landing_gear_entries`].
pub const LANDING_GEAR_SHIP: &str = "Gear_Ship";

const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

fn node(name: &str, parent: Option<u16>, x: f32, kind: u16) -> TestNode {
    let mut local = IDENTITY;
    local[0][3] = x;
    TestNode {
        name: name.to_owned(),
        parent,
        local,
        kind,
    }
}

fn hull(root: &str, bone: &str, x: f32) -> TestModel {
    TestModel::cube().with_nodes(vec![
        node(root, None, 0.0, 2),
        node(&format!("{root}_mesh"), Some(0), 0.0, 0),
        node(bone, Some(0), x, 2),
    ])
}

fn entity(guid: u128, class_name: &str, model: &str) -> TestEntity {
    TestEntity::new(guid, class_name, format!("entities/{class_name}.xml"))
        .geometry(format!("Objects/Test/{model}.cgf"), None)
        .vehicle("", "")
}

fn database() -> DataForgeBuilder {
    let ship = entity(0x1, LANDING_GEAR_SHIP, "ship")
        .landing_system(0x10)
        .loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("hardpoint_tender", "Gear_Tender"),
            TestLoadoutEntry::class("hardpoint_hangar", "Gear_Snub"),
        ]));
    let tender = entity(0x2, "Gear_Tender", "tender").landing_system(0x11);
    let snub = entity(0x3, "Gear_Snub", "tender")
        .attach_type("NOITEM_Vehicle_Snub")
        .landing_system(0x12);
    let ship_gear = landing_system_record(
        0x10,
        "Gear_Ship_LandingSystem",
        &[
            ("hardpoint_gear_front", "Objects/Test/gear.cgf", 10.0),
            ("hardpoint_gear_missing", "Objects/Test/gear.cgf", 10.0),
            ("hardpoint_gear_none", "", 10.0),
        ],
    );
    let tender_gear = landing_system_record(
        0x11,
        "Gear_Tender_LandingSystem",
        &[("bone_tender", "Objects/Test/gear.cgf", 5.0)],
    );
    let snub_gear = landing_system_record(
        0x12,
        "Gear_Snub_LandingSystem",
        &[("bone_tender", "Objects/Test/gear.cgf", 5.0)],
    );
    [ship, tender, snub]
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .record(ship_gear)
        .record(tender_gear)
        .record(snub_gear)
}

/// Returns the Entries of a synthetic Archive that holds a ship with landing gear.
///
/// `Gear_Ship` has a model with the helper `hardpoint_gear_front` at (5, 0, 0) and three gears:
/// `hardpoint_gear_front`, whose model is in the Archive, `hardpoint_gear_missing`, a bone the
/// model lacks, and `hardpoint_gear_none`, which names no model. Its Loadout puts `Gear_Tender`
/// into `hardpoint_tender` and the docked vehicle `Gear_Snub` into `hardpoint_hangar`; both have
/// a model with the helper `bone_tender` at (2, 0, 0) and one gear at that bone.
#[must_use]
pub fn landing_gear_entries() -> Vec<TestEntry> {
    let ship = hull("ship", "hardpoint_gear_front", 5.0);
    let tender = hull("tender", "bone_tender", 2.0);
    let gear = TestModel::cube().scaled(0.2);
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database().build()),
        TestEntry::stored("Data\\Objects\\Test\\ship.cgf", ship.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\ship.cgfm", ship.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\tender.cgf", tender.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\tender.cgfm", tender.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\gear.cgf", gear.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\gear.cgfm", gear.cgfm()),
    ]
}
