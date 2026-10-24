// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Assembly of the synthetic ship with exclusions and replacements applied.

use crate::assembly_support;

use assembly_support::{TestResult, entries, fixture};
use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, LoadoutChanges, LoadoutResolver, Part,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{CheckedGlb, check_glb};

fn assemble_with(changes: &LoadoutChanges) -> Result<Assembly, Box<dyn std::error::Error>> {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find("TEST_Ship")?;
    let tree = resolver.resolve_with(&record, changes)?;
    let models = ArchiveModels::new(&archive);
    Ok(Assembly::build(
        &tree,
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn names(assembly: &Assembly) -> Vec<&str> {
    assembly.parts().iter().map(Part::name).collect()
}

fn glb_of(assembly: &Assembly) -> Result<CheckedGlb, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    assembly.write_glb(&GlbOptions::default(), &mut bytes)?;
    Ok(check_glb(&bytes)?)
}

#[test]
fn an_excluded_branch_is_no_part_and_no_issue() -> TestResult {
    let assembly = assemble_with(&LoadoutChanges::new().exclude("hardpoint_weapon_nose"))?;

    assert_eq!(
        names(&assembly),
        [
            "TEST_Ship",
            "hardpoint_relay/TEST_Relay",
            "hardpoint_seat/TEST_Seat",
            "hardpoint_door/TEST_Door",
        ],
        "neither the mount nor the gun below it"
    );
    assert!(
        !assembly
            .issues()
            .iter()
            .any(|issue| issue.path().starts_with("hardpoint_weapon_nose")),
        "{:?}",
        assembly.issues()
    );
    Ok(())
}

#[test]
fn the_glb_of_an_assembly_without_the_branch_lacks_its_nodes() -> TestResult {
    let assembly = assemble_with(
        &LoadoutChanges::new()
            .exclude("hardpoint_weapon_nose")
            .exclude("hardpoint_relay"),
    )?;

    let glb = glb_of(&assembly)?;

    let named = |name: &str| glb.document.nodes().any(|node| node.name() == Some(name));
    assert!(named("TEST_Ship"));
    assert!(!named("hardpoint_weapon_nose/TEST_Mount"));
    assert!(!named("hardpoint_class_2/TEST_Gun"));
    assert!(!named("hardpoint_relay/TEST_Relay"));
    Ok(())
}

#[test]
fn a_replacement_is_placed_at_the_port_of_the_replaced_part() -> TestResult {
    let assembly = assemble_with(
        &LoadoutChanges::new().replace("hardpoint_weapon_nose/hardpoint_class_2", "TEST_Relay"),
    )?;

    assert_eq!(
        names(&assembly)[..3],
        [
            "TEST_Ship",
            "hardpoint_weapon_nose/TEST_Mount",
            "hardpoint_class_2/TEST_Relay"
        ]
    );
    let part = assembly
        .part_at("hardpoint_weapon_nose/hardpoint_class_2")
        .ok_or("the replacement")?;
    assert_eq!(part.class_name(), "TEST_Relay");
    assert!(part.model().is_some(), "its model is read");
    assert_eq!(
        part.world().translation(),
        [0.0, 4.0, 1.5],
        "at the helper the port names, as the gun was"
    );
    Ok(())
}
