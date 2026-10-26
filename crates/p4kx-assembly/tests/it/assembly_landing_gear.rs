// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The landing gear of a ship become parts placed at their bone of the ship's model.

use crate::assembly_support::{TestResult, assemble, fixture};

use p4kx_assembly::{Assembly, Part, Placement};
use p4kx_extraction::Archive;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestModel, TestNode, entity_schema,
    landing_system_record,
};

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

fn entries() -> Vec<TestEntry> {
    let ship = TestEntity::new(0x1, "Gear_Ship", "entities/gear_ship.xml")
        .geometry("Objects/Test/ship.cgf", None)
        .vehicle("", "")
        .landing_system(0x10);
    let system = landing_system_record(
        0x10,
        "Gear_Ship_LandingSystem",
        &[
            ("hardpoint_gear_front", "Objects/Test/gear.cgf", 10.0),
            ("hardpoint_gear_missing", "Objects/Test/gear.cgf", 10.0),
            ("hardpoint_gear_none", "", 10.0),
        ],
    );
    let database = entity_schema(DataForgeBuilder::new())
        .record(ship.record())
        .record(system)
        .build();
    let hull = TestModel::cube().with_nodes(vec![
        node("hull", None, 0.0, 2),
        node("hull_mesh", Some(0), 0.0, 0),
        node("hardpoint_gear_front", Some(0), 5.0, 2),
    ]);
    let gear = TestModel::cube().scaled(0.2);
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database),
        TestEntry::stored("Data\\Objects\\Test\\ship.cgf", hull.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\ship.cgfm", hull.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\gear.cgf", gear.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\gear.cgfm", gear.cgfm()),
    ]
}

fn built() -> Result<Assembly, Box<dyn std::error::Error>> {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;
    assemble(&archive, "Gear_Ship", 0)
}

#[test]
fn a_gear_is_a_part_with_its_model_at_the_bone_of_the_ships_model() -> TestResult {
    let assembly = built()?;

    let names: Vec<&str> = assembly.parts().iter().map(Part::name).collect();
    assert_eq!(
        names,
        [
            "Gear_Ship",
            "landing_gear/hardpoint_gear_front",
            "landing_gear/hardpoint_gear_missing",
            "landing_gear/hardpoint_gear_none",
        ]
    );
    let gear = assembly
        .part_at("landing_gear/hardpoint_gear_front")
        .ok_or("the front gear")?;
    assert_eq!(gear.parent(), Some(0));
    assert!(gear.model().is_some(), "the gear model is read");
    assert_eq!(
        gear.placement(),
        &Placement::Helper("hardpoint_gear_front".to_owned())
    );
    assert_eq!(gear.world().translation(), [5.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn a_gear_whose_bone_the_model_lacks_sits_at_the_origin_and_is_reported() -> TestResult {
    let assembly = built()?;

    let gear = assembly
        .part_at("landing_gear/hardpoint_gear_missing")
        .ok_or("the missing gear")?;
    assert_eq!(gear.placement(), &Placement::Origin);
    assert_eq!(gear.world().translation(), [0.0, 0.0, 0.0]);
    let codes: Vec<(&str, &str)> = assembly
        .issues()
        .iter()
        .map(|issue| (issue.path(), issue.kind().code()))
        .collect();
    assert!(
        codes.contains(&("landing_gear/hardpoint_gear_missing", "helper_not_found")),
        "{codes:?}"
    );
    assert!(
        codes.contains(&("landing_gear/hardpoint_gear_none", "no_geometry")),
        "{codes:?}"
    );
    Ok(())
}
