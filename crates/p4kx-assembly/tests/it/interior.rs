// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of the synthetic socpak: placements, composed transforms, nested containers,
//! cycles, unresolved parts and the GLB.

use std::path::Path;

use p4kx_assembly::{
    Interior, InteriorOptions, InteriorPart, InteriorPartKind, Issue, IssueKind, MAX_INTERIOR_DEPTH,
};
use p4kx_extraction::{Archive, select_entries};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::gltf::Node;
use p4kx_testkit::{
    INTERIOR_ROOT, SOC_IDENTITY, TestContainer, TestEntry, TestObjectContainer, TestSoc, check_glb,
    interior_entries,
};

use crate::assembly_support::{TestResult, fixture};

fn build_in(
    path: &Path,
    options: &InteriorOptions,
) -> Result<Interior, Box<dyn std::error::Error>> {
    let archive = Archive::open(path)?;
    let found = select_entries(&archive, &[INTERIOR_ROOT])?;
    let entry = found.first().ok_or("no root container")?;
    let root = archive.open_container(entry)?;
    Ok(Interior::build(&[&archive, &root], INTERIOR_ROOT, options)?)
}

fn sample(options: &InteriorOptions) -> Result<Interior, Box<dyn std::error::Error>> {
    let fixture = fixture(interior_entries())?;
    build_in(&fixture.path, options)
}

fn named<'a>(interior: &'a Interior, name: &str) -> Result<&'a InteriorPart, String> {
    interior
        .parts()
        .iter()
        .find(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))
}

fn close(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-4)
}

fn issue<'a>(interior: &'a Interior, code: &str) -> Vec<&'a Issue> {
    interior
        .issues()
        .iter()
        .filter(|issue| issue.kind().code() == code)
        .collect()
}

#[test]
fn nested_containers_are_placed_with_their_transforms_composed() -> TestResult {
    let interior = sample(&InteriorOptions::default())?;

    let cockpit = named(&interior, "cockpit")?;
    assert_eq!(cockpit.kind(), InteriorPartKind::Container);
    assert!(close(&cockpit.world().translation(), &[0.0, 5.0, 0.0]));
    let seat = named(&interior, "cockpit_seat")?;
    assert!(
        close(&seat.world().translation(), &[0.0, 5.5, 0.0]),
        "(0.5, 0, 0) turned a quarter about Z lies on Y: {:?}",
        seat.world()
    );
    let cube = interior
        .parts()
        .iter()
        .rev()
        .find(|part| part.model_path() == Some("objects/test/cube.cgf"))
        .ok_or("no cube in the cockpit")?;
    assert!(close(&cube.world().translation(), &[0.0, 6.0, 0.0]));
    assert!(
        close(&cube.world().rows[0], &[0.0, -1.0, 0.0, 0.0]),
        "the cube turns with the cockpit: {:?}",
        cube.world()
    );
    assert!(close(&cube.local().translation(), &[1.0, 0.0, 0.0]));
    Ok(())
}

#[test]
fn the_parts_hang_in_the_container_hierarchy() -> TestResult {
    let interior = sample(&InteriorOptions::default())?;

    let root = interior.root().ok_or("no root")?;
    assert_eq!(
        (root.name(), root.kind(), root.parent()),
        ("ship_interior", InteriorPartKind::Container, None)
    );
    let streams: Vec<usize> = interior
        .parts()
        .iter()
        .enumerate()
        .filter(|(_, part)| part.kind() == InteriorPartKind::Stream)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(streams.len(), 2, "the root's and the cockpit's .soc file");
    for (index, part) in interior.parts().iter().enumerate() {
        if let Some(parent) = part.parent() {
            assert!(parent < index, "{} comes after its parent", part.name());
        }
        if part.kind() == InteriorPartKind::Model {
            let parent = part.parent().and_then(|p| interior.parts().get(p));
            assert_eq!(
                parent.map(InteriorPart::kind),
                Some(InteriorPartKind::Stream)
            );
        }
    }
    assert_eq!(
        interior.containers(),
        4,
        "the root, the cockpit and both rings"
    );
    assert_eq!(interior.model_parts(), 6);
    assert_eq!(interior.modelled_parts(), 5, "all but the absent model");
    Ok(())
}

#[test]
fn a_model_comes_from_the_container_first_and_equal_models_are_shared() -> TestResult {
    let interior = sample(&InteriorOptions::default())?;

    let seat = named(&interior, "cockpit_seat")?;
    assert!(seat.model().is_some(), "only the cockpit holds this model");
    let cubes: Vec<&InteriorPart> = interior
        .parts()
        .iter()
        .filter(|part| part.model_path() == Some("objects/test/cube.cgf"))
        .collect();
    assert_eq!(cubes.len(), 3);
    assert!(
        cubes.iter().all(|cube| cube
            .materials()
            .library()
            .is_some_and(|l| l.materials.len() == 3)),
        "the library the cube names is read"
    );
    Ok(())
}

#[test]
fn what_cannot_be_resolved_is_listed() -> TestResult {
    let interior = sample(&InteriorOptions::default())?;

    let missing = issue(&interior, "container_missing");
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(missing.iter().any(
        |i| matches!(i.kind(), IssueKind::ContainerMissing { name } if name == "modules/absent")
    ));
    let cycles = issue(&interior, "container_cycle");
    assert_eq!(
        cycles.len(),
        2,
        "the root nested in the cockpit and the rings: {cycles:?}"
    );
    let paths: Vec<&str> = cycles.iter().map(|i| i.path()).collect();
    assert!(paths.contains(&"ship_interior/cockpit"), "{paths:?}");
    assert!(paths.contains(&"ship_interior/ring_a/ring_b"), "{paths:?}");
    let models = issue(&interior, "geometry_missing");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].name(), "absent");
    assert_eq!(models[0].path(), "ship_interior");
    assert_eq!(issue(&interior, "invalid_placement").len(), 1);
    Ok(())
}

#[test]
fn the_glb_holds_a_node_per_part_and_shares_the_meshes_of_equal_models() -> TestResult {
    let interior = sample(&InteriorOptions::default())?;
    let mut bytes = Vec::new();

    let summary = interior.write_glb(&GlbOptions::default(), &mut bytes)?;

    let glb = check_glb(&bytes)?;
    let node = |name: &str| -> Result<Node<'_>, String> {
        glb.document
            .nodes()
            .find(|node| node.name() == Some(name))
            .ok_or_else(|| format!("no node {name}"))
    };
    let cockpit = node("cockpit")?;
    let (translation, _, _) = cockpit.transform().decomposed();
    assert!(
        close(&translation, &[0.0, 0.0, -5.0]),
        "Y up: the cockpit lies 5 m along the old Y axis, now -Z: {translation:?}"
    );
    let root = node("ship_interior")?;
    assert!(
        root.children()
            .any(|child| child.index() == cockpit.index())
    );
    assert_eq!(summary.parts, 5);
    assert_eq!(summary.shared_meshes, 2, "the three cubes share one mesh");
    Ok(())
}

#[test]
fn the_level_of_detail_applies_to_every_model() -> TestResult {
    let coarse = sample(&InteriorOptions::default().with_lod(1))?;
    let fine = sample(&InteriorOptions::default())?;

    let lod_of = |interior: &Interior| -> Result<u32, String> {
        let model = named(interior, "lodded")?.model().ok_or("no model")?;
        model
            .lods
            .first()
            .map(|lod| lod.level)
            .ok_or_else(|| "no lod".to_owned())
    };
    assert_eq!(lod_of(&fine)?, 0);
    assert_eq!(lod_of(&coarse)?, 1);
    assert_eq!(coarse.lod(), 1);
    Ok(())
}

#[test]
fn the_parts_and_vertices_are_limited() -> TestResult {
    let few = sample(&InteriorOptions::default().with_limits(5, usize::MAX))?;
    let small = sample(&InteriorOptions::default().with_limits(usize::MAX, 10))?;

    assert_eq!(few.parts().len(), 5);
    assert_eq!(issue(&few, "too_many_parts").len(), 1);
    assert!(issue(&small, "too_many_vertices").len() >= 1);
    assert!(small.modelled_parts() < 5);
    Ok(())
}

fn archive_with(
    root: &TestContainer,
) -> Result<crate::assembly_support::Fixture, Box<dyn std::error::Error>> {
    fixture(vec![TestEntry::stored(INTERIOR_ROOT, root.build())])
}

#[test]
fn corrupt_descriptions_and_soc_files_are_issues_not_panics() -> TestResult {
    let mut corrupt = TestSoc::new()
        .model("a.cgf")
        .object(0, SOC_IDENTITY)
        .bytes();
    corrupt.truncate(corrupt.len() / 2);
    let root = TestContainer::new()
        .entry(TestEntry::stored(
            "ship_interior.xml",
            TestObjectContainer::new("x")
                .stream("one.soc")
                .stream("two.soc")
                .stream("missing.soc")
                .stream("../escape.soc")
                .xml(),
        ))
        .entry(TestEntry::stored("one.soc", corrupt))
        .entry(TestEntry::stored("two.soc", b"not a chunk file".to_vec()));
    let fixture = archive_with(&root)?;

    let interior = build_in(&fixture.path, &InteriorOptions::default())?;

    assert_eq!(issue(&interior, "file_unreadable").len(), 2);
    assert_eq!(issue(&interior, "file_missing").len(), 2);
    assert_eq!(interior.model_parts(), 0);

    let broken = TestContainer::new().entry(TestEntry::stored(
        "ship_interior.xml",
        b"<ObjectContainer".to_vec(),
    ));
    let fixture = archive_with(&broken)?;
    let interior = build_in(&fixture.path, &InteriorOptions::default())?;
    assert_eq!(issue(&interior, "file_unreadable").len(), 1);
    let empty = archive_with(&TestContainer::new())?;
    let interior = build_in(&empty.path, &InteriorOptions::default())?;
    assert_eq!(issue(&interior, "file_missing").len(), 1);
    assert_eq!(interior.parts().len(), 1);
    Ok(())
}

fn chain_container(name: &str, next: Option<String>) -> TestContainer {
    let mut description = TestObjectContainer::new(name);
    if let Some(next) = next {
        description = description.child(&next, [0.0, 1.0, 0.0], [1.0, 0.0, 0.0, 0.0]);
    }
    TestContainer::new().entry(TestEntry::stored(format!("{name}.xml"), description.xml()))
}

#[test]
fn nesting_beyond_the_depth_limit_is_refused() -> TestResult {
    let levels = MAX_INTERIOR_DEPTH + 4;
    let mut entries = vec![TestEntry::stored(
        INTERIOR_ROOT,
        chain_container(
            "ship_interior",
            Some("ObjectContainers/chain/level_0".to_owned()),
        )
        .build(),
    )];
    for level in 0..levels {
        let next =
            (level + 1 < levels).then(|| format!("ObjectContainers/chain/level_{}", level + 1));
        entries.push(TestEntry::stored(
            format!("Data\\ObjectContainers\\chain\\level_{level}.socpak"),
            chain_container(&format!("level_{level}"), next).build(),
        ));
    }
    let fixture = fixture(entries)?;

    let interior = build_in(&fixture.path, &InteriorOptions::default())?;

    assert_eq!(
        issue(&interior, "too_deep").len(),
        1,
        "{:?}",
        interior.issues()
    );
    assert_eq!(interior.containers(), MAX_INTERIOR_DEPTH);
    let deepest = interior.parts().last().ok_or("no parts")?;
    assert!(close(
        &deepest.world().translation(),
        &[0.0, (MAX_INTERIOR_DEPTH - 1) as f32, 0.0]
    ));
    Ok(())
}

#[test]
fn an_unreadable_container_is_an_issue_of_its_name() -> TestResult {
    let root = TestContainer::new()
        .entry(TestEntry::stored(
            "ship_interior.xml",
            TestObjectContainer::new("ship")
                .child("broken", [0.0; 3], [1.0, 0.0, 0.0, 0.0])
                .xml(),
        ))
        .entry(TestEntry::stored(
            "broken.socpak",
            b"not a container".to_vec(),
        ));
    let fixture = archive_with(&root)?;

    let interior = build_in(&fixture.path, &InteriorOptions::default())?;

    let issues = issue(&interior, "container_unreadable");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].name(), "broken");
    Ok(())
}
