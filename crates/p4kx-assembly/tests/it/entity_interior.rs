// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of an Entity: its object containers found in the Record, placed relative to the
//! hull, grouped under an Interior node and listed when unresolved.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, ContainerSource, EntityInterior, EntityNode,
    INTERIOR_NODE, InteriorOptions, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::gltf::Node;
use p4kx_testkit::{
    CheckedGlb, ENTITY_INTERIOR_CLASS, ENTITY_INTERIOR_FILE, check_glb, entity_interior_entries,
};

use crate::assembly_support::{TestResult, fixture};

struct Built {
    tree: EntityNode,
    assembly: Assembly,
    interior: EntityInterior,
}

fn build() -> Result<Built, Box<dyn Error>> {
    let fixture = fixture(entity_interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ENTITY_INTERIOR_CLASS)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    let interior = EntityInterior::build(&archive, &assembly, &tree, &InteriorOptions::default())?;
    Ok(Built {
        tree,
        assembly,
        interior,
    })
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
fn the_entitys_records_name_its_containers_with_bone_and_offset() -> TestResult {
    let built = build()?;

    let containers = built.tree.object_containers();
    assert_eq!(containers.len(), 5);
    let front = &containers[0];
    assert_eq!(front.file(), ENTITY_INTERIOR_FILE);
    assert_eq!(front.bone(), "hp_int_front");
    assert!(close(&front.position(), &[0.0, 6.0, 0.0]));
    assert_eq!(front.source(), ContainerSource::Vehicle);
    let item = &containers[4];
    assert_eq!(item.source(), ContainerSource::Item);
    assert_eq!(item.bone(), "");
    Ok(())
}

#[test]
fn a_container_is_placed_at_its_bone_moved_by_the_offset_but_never_twice() -> TestResult {
    let built = build()?;

    let placed: Vec<[f32; 3]> = built
        .interior
        .containers()
        .iter()
        .map(|container| container.placement().translation())
        .collect();
    assert_eq!(placed.len(), 4, "the missing container is left out");
    assert!(
        close(&placed[0], &[0.0, 6.0, 0.0]),
        "the bone already is at the offset: {:?}",
        placed[0]
    );
    assert!(
        close(&placed[1], &[1.0, -3.0, 0.0]),
        "the offset moves the bone: {:?}",
        placed[1]
    );
    assert!(
        close(&placed[2], &[0.0, 0.0, 10.0]),
        "no bone, the offset from the origin: {:?}",
        placed[2]
    );
    assert!(
        close(&placed[3], &[0.0; 3]),
        "the item's container sits at the origin: {:?}",
        placed[3]
    );
    Ok(())
}

#[test]
fn the_interior_models_lie_where_the_container_and_the_hull_put_them() -> TestResult {
    let built = build()?;

    let container = &built.interior.containers()[1];
    let seat = container
        .interior()
        .parts()
        .iter()
        .find(|part| part.name() == "cockpit_seat")
        .ok_or("no seat")?;
    let world = container.placement().then(seat.world()).translation();
    assert!(
        close(&world, &[1.0, 2.5, 0.0]),
        "(0.5, 0, 0) in the cockpit at (0, 5, 0) turned a quarter, in the container at (1, -3, 0): {world:?}"
    );
    assert!(built.interior.modelled_parts() > 0);
    Ok(())
}

#[test]
fn unresolved_parts_are_listed_with_their_container() -> TestResult {
    let built = build()?;

    let codes: Vec<(&str, &str)> = built
        .interior
        .issues()
        .iter()
        .map(|issue| (issue.path(), issue.kind().code()))
        .collect();
    assert!(
        codes.contains(&(INTERIOR_NODE, "container_missing")),
        "the absent file: {codes:?}"
    );
    assert!(
        codes.contains(&(INTERIOR_NODE, "helper_not_found")),
        "the absent bone: {codes:?}"
    );
    assert!(
        codes
            .iter()
            .any(|(path, code)| path.starts_with("ship_interior") && *code == "container_cycle"),
        "the nested rings: {codes:?}"
    );
    assert!(
        codes.iter().any(|(_, code)| *code == "file_missing"
            || *code == "model_missing"
            || *code == "geometry_missing"),
        "the absent model: {codes:?}"
    );
    Ok(())
}

#[test]
fn the_assembly_without_an_interior_is_the_hull_alone() -> TestResult {
    let built = build()?;

    assert!(built.assembly.interior().is_none());
    let glb = glb(&built.assembly)?;
    assert!(
        glb.document
            .nodes()
            .all(|node| node.name() != Some(INTERIOR_NODE))
    );
    assert!(
        glb.document
            .nodes()
            .all(|node| node.name() != Some("ship_interior")),
        "none of the rooms"
    );
    Ok(())
}

#[test]
fn the_glb_groups_the_interior_below_an_interior_node_of_the_hull() -> TestResult {
    let built = build()?;
    let issues = built.assembly.issues().len();
    let interior_issues = built.interior.issues().len();
    let assembly = built.assembly.with_interior(built.interior);

    assert_eq!(assembly.issues().len(), issues + interior_issues);
    let glb = glb(&assembly)?;
    let hull = named(&glb, ENTITY_INTERIOR_CLASS)?;
    let group = named(&glb, INTERIOR_NODE)?;
    assert!(
        hull.children().any(|child| child.index() == group.index()),
        "the Interior node hangs below the hull"
    );
    let rooms: Vec<Node<'_>> = group.children().collect();
    assert_eq!(rooms.len(), 4, "one node per container");
    let front = rooms[0].transform().decomposed().0;
    assert!(
        close(&front, &[0.0, 0.0, -6.0]),
        "(0, 6, 0) in Z up is (0, 0, -6) in Y up: {front:?}"
    );
    let aft = rooms[1].transform().decomposed().0;
    assert!(close(&aft, &[1.0, 0.0, 3.0]), "(1, -3, 0) in Z up: {aft:?}");
    assert!(glb.document.meshes().count() >= 2, "the hull and the cubes");
    Ok(())
}
