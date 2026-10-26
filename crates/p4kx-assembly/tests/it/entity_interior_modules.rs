// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of an Assembly with attached Entities: their containers sit relative to them, a
//! docked vehicle brings its containers only when it is expanded, and the tree must be the
//! Assembly's own.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyError, AssemblyOptions, EntityContainer, EntityInterior,
    INTERIOR_NODE, InteriorOptions, Issue, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::gltf::Node;
use p4kx_testkit::{
    CheckedGlb, DataForgeBuilder, ENTITY_INTERIOR_FILE, ENTITY_MODULE_CARRIER, ENTITY_MODULE_CLASS,
    TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestPort, check_glb,
    entity_module_entries, entity_schema, lit_interior_entries,
};

use crate::assembly_support::{TestResult, fixture};

struct Carrier {
    assembly: Assembly,
    interior: EntityInterior,
}

fn carrier() -> Result<Carrier, Box<dyn Error>> {
    let fixture = fixture(entity_module_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ENTITY_MODULE_CARRIER)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    let interior = EntityInterior::build(&archive, &assembly, &tree, &InteriorOptions::default())?;
    Ok(Carrier { assembly, interior })
}

fn close(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-4)
}

fn glb(assembly: &Assembly) -> Result<CheckedGlb, Box<dyn Error>> {
    let mut bytes = Vec::new();
    assembly.write_glb(&GlbOptions::default(), &mut bytes)?;
    Ok(check_glb(&bytes)?)
}

fn named<'a>(glb: &'a CheckedGlb, name: &str) -> Result<Node<'a>, String> {
    glb.document
        .nodes()
        .find(|node| node.name() == Some(name))
        .ok_or_else(|| format!("no node {name}"))
}

#[test]
fn the_containers_of_an_attached_entity_sit_relative_to_that_entity() -> TestResult {
    let built = carrier()?;

    let owners: Vec<usize> = built
        .interior
        .containers()
        .iter()
        .map(EntityContainer::part)
        .collect();
    assert_eq!(owners, [0, 1, 1], "the carrier's, then the module's two");
    let module = &built.assembly.parts()[1];
    assert_eq!(module.class_name(), ENTITY_MODULE_CLASS);
    let bone = built.interior.containers()[1].placement().translation();
    assert!(close(&bone, &[0.0, 2.0, 0.0]), "{bone:?}");
    let world = module
        .world()
        .then(built.interior.containers()[1].placement())
        .translation();
    assert!(close(&world, &[10.0, 2.0, 0.0]), "{world:?}");
    Ok(())
}

#[test]
fn the_issues_of_an_attached_entity_carry_its_port_path() -> TestResult {
    let built = carrier()?;

    let missing: Vec<&str> = built
        .interior
        .issues()
        .iter()
        .filter(|issue| issue.kind().code() == "helper_not_found")
        .map(Issue::path)
        .collect();
    assert_eq!(missing, ["hardpoint_module/Interior"]);
    let nested: Vec<&str> = built
        .interior
        .issues()
        .iter()
        .filter(|issue| issue.kind().code() == "container_cycle")
        .map(Issue::path)
        .collect();
    assert!(
        nested
            .iter()
            .any(|path| path.starts_with("hardpoint_module/ship_interior")),
        "{nested:?}"
    );
    Ok(())
}

#[test]
fn the_glb_hangs_an_interior_node_below_each_entity_with_containers() -> TestResult {
    let built = carrier()?;
    let assembly = built.assembly.with_interior(built.interior);

    let glb = glb(&assembly)?;
    let groups: Vec<Node<'_>> = glb
        .document
        .nodes()
        .filter(|node| node.name() == Some(INTERIOR_NODE))
        .collect();
    assert_eq!(groups.len(), 2);
    let carrier = named(&glb, ENTITY_MODULE_CARRIER)?;
    let module = named(&glb, &format!("hardpoint_module/{ENTITY_MODULE_CLASS}"))?;
    for owner in [&carrier, &module] {
        assert!(
            groups
                .iter()
                .any(|group| owner.children().any(|child| child.index() == group.index())),
            "an Interior node below {:?}",
            owner.name()
        );
    }
    let below_module = module
        .children()
        .find(|child| child.name() == Some(INTERIOR_NODE))
        .ok_or("no Interior below the module")?;
    assert_eq!(below_module.children().count(), 2);
    Ok(())
}

#[test]
fn a_tree_other_than_the_assemblys_is_refused() -> TestResult {
    let fixture = fixture(entity_module_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let carrier = resolver.resolve(&resolver.entities().find(ENTITY_MODULE_CARRIER)?)?;
    let module = resolver.resolve(&resolver.entities().find(ENTITY_MODULE_CLASS)?)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&carrier, &models, &AssemblyOptions::default())?;

    let refused = EntityInterior::build(&archive, &assembly, &module, &InteriorOptions::default());

    assert!(matches!(refused, Err(AssemblyError::TreeMismatch)));
    Ok(())
}

fn interior_of(
    entities: &[TestEntity],
    class_name: &str,
    stop_at_docked: bool,
) -> Result<EntityInterior, Box<dyn Error>> {
    let database = entities
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        });
    let mut entries = vec![TestEntry::zstd("Data\\Game2.dcb", database.build())];
    entries.extend(lit_interior_entries());
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    if stop_at_docked {
        resolver = resolver.stopping_at_docked_vehicles();
    }
    let tree = resolver.resolve(&resolver.entities().find(class_name)?)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    Ok(EntityInterior::build(
        &archive,
        &assembly,
        &tree,
        &InteriorOptions::default(),
    )?)
}

#[test]
fn a_non_finite_offset_is_an_invalid_offset_not_a_mismatch() -> TestResult {
    let ship = TestEntity::new(0x21, "TEST_Nan", "entities/test_nan.xml").object_container(
        ENTITY_INTERIOR_FILE,
        "",
        [f32::NAN, 0.0, 0.0],
        [0.0; 3],
    );

    let interior = interior_of(&[ship], "TEST_Nan", false)?;

    assert_eq!(interior.containers().len(), 1);
    assert!(
        interior
            .issues()
            .iter()
            .any(|issue| issue.kind().code() == "invalid_offset")
    );
    Ok(())
}

fn fleet() -> Vec<TestEntity> {
    vec![
        TestEntity::new(0x31, "TEST_Mothership", "entities/test_mothership.xml")
            .attach_type("NOITEM_Vehicle")
            .object_container(ENTITY_INTERIOR_FILE, "", [0.0; 3], [0.0; 3])
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "hangar",
                "TEST_Snub",
            )])),
        TestEntity::new(0x32, "TEST_Snub", "entities/test_snub.xml")
            .attach_type("NOITEM_Vehicle_Snub")
            .object_container(ENTITY_INTERIOR_FILE, "", [0.0; 3], [0.0; 3]),
    ]
}

#[test]
fn a_docked_vehicle_that_is_not_expanded_brings_no_interior() -> TestResult {
    let interior = interior_of(&fleet(), "TEST_Mothership", true)?;

    let owners: Vec<usize> = interior
        .containers()
        .iter()
        .map(EntityContainer::part)
        .collect();
    assert_eq!(owners, [0]);
    Ok(())
}

#[test]
fn a_hidden_part_brings_no_interior() -> TestResult {
    let entities = [
        TestEntity::new(0x41, "TEST_Host", "entities/test_host.xml")
            .object_container(ENTITY_INTERIOR_FILE, "", [0.0; 3], [0.0; 3])
            .port(
                TestPort::new("hardpoint_hidden")
                    .flags("invisible")
                    .default_item(0x42),
            ),
        TestEntity::new(0x42, "TEST_Module", "entities/test_module.xml").object_container(
            ENTITY_INTERIOR_FILE,
            "",
            [0.0; 3],
            [0.0; 3],
        ),
    ];

    let interior = interior_of(&entities, "TEST_Host", false)?;

    let owners: Vec<usize> = interior
        .containers()
        .iter()
        .map(EntityContainer::part)
        .collect();
    assert_eq!(owners, [0]);
    assert!(
        !interior
            .issues()
            .iter()
            .any(|issue| issue.path().starts_with("hardpoint_hidden"))
    );
    Ok(())
}

#[test]
fn an_expanded_docked_vehicle_brings_its_interior() -> TestResult {
    let interior = interior_of(&fleet(), "TEST_Mothership", false)?;

    let owners: Vec<usize> = interior
        .containers()
        .iter()
        .map(EntityContainer::part)
        .collect();
    assert_eq!(owners, [0, 1]);
    Ok(())
}
