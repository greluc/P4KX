// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Items on ports the vehicle implementation or the port definition marks `invisible` are kept as
//! hidden parts that are not drawn.

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver, PortVisibility};
use p4kx_dataforge::DataForge;
use p4kx_extraction::Archive;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestModel, TestPort, entity_schema};

use crate::assembly_support::{TestResult, fixture};
use crate::vehicle_support::{DEFINITION, standard_files};

const SHIP: u128 = 0x1000;
const SKID: u128 = 0x2000;
const BOLT: u128 = 0x3000;

fn build(modification: &str) -> Result<Assembly, Box<dyn std::error::Error>> {
    let bolt = TestEntity::new(BOLT, "TEST_Bolt", "entities/test_bolt.xml")
        .geometry("Objects/Test/cube.cgf", None);
    let skid = TestEntity::new(SKID, "TEST_Skid", "entities/test_skid.xml")
        .geometry("Objects/Test/cube.cgf", None)
        .port(TestPort::new("inner").default_item(BOLT));
    let ship = TestEntity::new(SHIP, "TEST_Ship", "entities/test_ship.xml")
        .geometry("Objects/Test/cube.cgf", None)
        .vehicle(DEFINITION, modification)
        .port(TestPort::new("hardpoint_gun").default_item(SKID))
        .port(TestPort::new("hardpoint_seat").default_item(SKID))
        .port(
            TestPort::new("hardpoint_def")
                .flags("invisible $editable")
                .default_item(SKID),
        );
    let bytes = entity_schema(DataForgeBuilder::new())
        .record(bolt.record())
        .record(skid.record())
        .record(ship.record())
        .build();
    let model = TestModel::cube();
    let fixture = fixture(vec![
        p4kx_testkit::TestEntry::zstd("Data\\Game2.dcb", bytes.clone()),
        p4kx_testkit::TestEntry::stored("Data\\Objects\\Test\\cube.cgf", model.cgf()),
        p4kx_testkit::TestEntry::stored("Data\\Objects\\Test\\cube.cgfm", model.cgfm()),
    ])?;
    let archive = Archive::open(&fixture.path)?;
    let forge = DataForge::open(&bytes)?;
    let files = standard_files();
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find("TEST_Ship")?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(&archive);
    Ok(Assembly::build(
        &tree,
        &models,
        &AssemblyOptions::default(),
    )?)
}

#[test]
fn items_on_invisible_ports_are_kept_but_not_drawn() -> TestResult {
    let assembly = build("")?;

    assert_eq!(assembly.parts().len(), 7, "every part stays in the list");
    let drawn: Vec<&str> = assembly
        .parts()
        .iter()
        .filter(|part| part.model().is_some())
        .map(p4kx_assembly::Part::path)
        .collect();
    assert_eq!(
        drawn,
        ["", "hardpoint_seat", "hardpoint_seat/inner"],
        "only the root and the visible port are drawn"
    );
    let hidden: Vec<&str> = assembly
        .parts()
        .iter()
        .filter(|part| part.is_hidden())
        .map(p4kx_assembly::Part::path)
        .collect();
    assert_eq!(
        hidden,
        [
            "hardpoint_gun",
            "hardpoint_gun/inner",
            "hardpoint_def",
            "hardpoint_def/inner"
        ],
        "the items below a hidden item are hidden with it"
    );
    assert_eq!(assembly.hidden_parts(), 4);
    assert!(
        assembly
            .issues()
            .iter()
            .all(|issue| issue.kind().code() == "material_library"),
        "hiding raises no issue: {:?}",
        assembly.issues()
    );
    Ok(())
}

#[test]
fn a_modification_that_clears_the_flag_draws_the_item_again() -> TestResult {
    let assembly = build("Open")?;

    let gun = assembly.part_at("hardpoint_gun").ok_or("the gun part")?;
    assert!(!gun.is_hidden() && gun.model().is_some());
    assert!(
        assembly
            .part_at("hardpoint_gun/inner")
            .is_some_and(|part| part.model().is_some())
    );
    assert_eq!(assembly.hidden_parts(), 2, "the definition's flag remains");
    Ok(())
}

#[test]
fn the_tree_keeps_the_visibility_of_a_port_the_definition_flags() -> TestResult {
    let bytes = entity_schema(DataForgeBuilder::new())
        .record(
            TestEntity::new(SHIP, "TEST_Ship", "entities/test_ship.xml")
                .port(TestPort::new("relay").flags("invisible"))
                .port(TestPort::new("plain"))
                .record(),
        )
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find("TEST_Ship")?;
    let tree = resolver.resolve(&record)?;
    let seen: Vec<(&str, PortVisibility)> = tree
        .ports()
        .iter()
        .map(|port| (port.name(), port.visibility()))
        .collect();
    assert_eq!(
        seen,
        [
            ("relay", PortVisibility::Invisible),
            ("plain", PortVisibility::Unspecified)
        ]
    );
    Ok(())
}
