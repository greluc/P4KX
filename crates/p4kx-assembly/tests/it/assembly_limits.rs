// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Limits and failures of building an Assembly: part, vertex and depth budgets and a missing key.

use crate::assembly_support;

use assembly_support::{TestResult, assemble, entries, fixture};
use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyError, AssemblyOptions, IssueKind, LoadoutError,
    LoadoutResolver, MAX_LOADOUT_DEPTH,
};
use p4kx_extraction::{Archive, ArchiveError, DataForgeEntry, archive_error_in};
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, entity_schema,
};

fn build(
    archive: &Archive,
    entity: &str,
    options: &AssemblyOptions,
) -> Result<Assembly, Box<dyn std::error::Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(entity)?;
    let tree = resolver.resolve(&record)?;
    Ok(Assembly::build(
        &tree,
        &ArchiveModels::new(archive),
        options,
    )?)
}

fn database_archive(
    entities: &[TestEntity],
) -> Result<assembly_support::Fixture, Box<dyn std::error::Error>> {
    let database = entities
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        });
    fixture(vec![TestEntry::zstd("Data\\Game2.dcb", database.build())])
}

#[test]
fn parts_beyond_the_limit_are_left_out_and_listed_once() -> TestResult {
    let leaf = TestEntity::new(0x2, "Leaf", "entities/leaf.xml");
    let entries: Vec<TestLoadoutEntry> = (0..20)
        .map(|n| TestLoadoutEntry::class(format!("port_{n:02}"), "Leaf"))
        .collect();
    let root =
        TestEntity::new(0x1, "Root", "entities/root.xml").loadout(TestLoadout::Manual(entries));
    let fixture = database_archive(&[root, leaf])?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = build(
        &archive,
        "Root",
        &AssemblyOptions::default().with_limits(10, usize::MAX),
    )?;

    assert_eq!(assembly.parts().len(), 10, "the root and nine ports");
    let capped: Vec<_> = assembly
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::TooManyParts { limit: 10 }))
        .collect();
    assert_eq!(capped.len(), 1, "reported once: {:?}", assembly.issues());
    assert_eq!(capped[0].path(), "port_09");
    Ok(())
}

#[test]
fn models_beyond_the_vertex_budget_are_left_out_in_part_order() -> TestResult {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = build(
        &archive,
        "TEST_Ship",
        &AssemblyOptions::default().with_limits(100, 60),
    )?;

    let without: Vec<&str> = assembly
        .parts()
        .iter()
        .filter(|part| part.geometry().is_some() && part.model().is_none())
        .map(p4kx_assembly::Part::path)
        .collect();
    assert_eq!(
        without,
        [
            "hardpoint_weapon_nose/hardpoint_class_2",
            "hardpoint_relay",
            "hardpoint_door"
        ],
        "the ship and the mount (24 vertices each) fit, the gun and the relay do not, the door is missing"
    );
    assert!(
        assembly
            .issues()
            .iter()
            .any(|issue| matches!(issue.kind(), IssueKind::TooManyVertices { limit: 60 })),
        "{:?}",
        assembly.issues()
    );
    Ok(())
}

#[test]
fn a_chain_deeper_than_the_loadout_depth_is_cut_and_listed() -> TestResult {
    let chain: Vec<TestEntity> = (0..MAX_LOADOUT_DEPTH + 8)
        .map(|n| {
            let entity = TestEntity::new(
                n as u128 + 1,
                format!("Link_{n}"),
                format!("entities/l{n}.xml"),
            );
            entity.loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "next",
                format!("Link_{}", n + 1),
            )]))
        })
        .collect();
    let fixture = database_archive(&chain)?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = build(&archive, "Link_0", &AssemblyOptions::default())?;

    assert!(
        assembly.parts().len() <= MAX_LOADOUT_DEPTH + 1,
        "{}",
        assembly.parts().len()
    );
    assert!(
        assembly
            .issues()
            .iter()
            .any(|issue| matches!(issue.kind(), IssueKind::Unresolved(LoadoutError::TooDeep))),
        "{:?}",
        assembly.issues()
    );
    Ok(())
}

#[test]
fn a_missing_archive_key_stops_the_build_instead_of_listing_every_part() -> TestResult {
    let encrypted: Vec<TestEntry> = entries()
        .into_iter()
        .map(|entry| {
            if format!("{entry:?}").contains("ship.cgfm") {
                TestEntry::stored("Data\\Objects\\Test\\ship.cgfm", Vec::<u8>::new()).encrypted()
            } else {
                entry
            }
        })
        .collect();
    let fixture = fixture(encrypted)?;
    let archive = Archive::open(&fixture.path)?;

    let error = assemble(&archive, "TEST_Ship", 0).err().ok_or("an error")?;

    let build_error = error
        .downcast_ref::<AssemblyError>()
        .ok_or("an AssemblyError")?;
    let source = std::error::Error::source(build_error).ok_or("a source")?;
    assert!(
        archive_error_in(source).is_some_and(ArchiveError::is_key_required),
        "the Archive error stays reachable: {source}"
    );
    Ok(())
}
