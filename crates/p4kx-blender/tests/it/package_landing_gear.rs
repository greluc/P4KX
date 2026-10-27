// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A landing gear of a ship is a package node of its own, drawn but not an Entity.

use p4kx_blender::{Manifest, NodeKind, PackageInfo, write_package};
use p4kx_extraction::Archive;
use p4kx_testkit::{LANDING_GEAR_SHIP, landing_gear_entries};

use crate::support::{TestResult, assemble, fixture, glb_options, never};

#[test]
fn a_gear_is_an_object_node_with_a_mesh_and_without_an_entity() -> TestResult {
    let fixture = fixture(landing_gear_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, LANDING_GEAR_SHIP)?;
    let models = p4kx_assembly::ArchiveModels::new(&archive);
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("ship");

    write_package(
        &assembly,
        &models,
        &models.textures(),
        &glb_options(0),
        &PackageInfo::new(None, None),
        &out,
        &never,
    )?;

    let manifest: Manifest =
        serde_json::from_str(&std::fs::read_to_string(out.join("scene.json"))?)?;
    let gear = manifest
        .nodes
        .iter()
        .find(|node| node.path == "landing_gear/hardpoint_gear_front")
        .ok_or("the gear node")?;
    assert_eq!(gear.kind, NodeKind::Object);
    assert_eq!(gear.name, "landing_gear/hardpoint_gear_front");
    assert!(gear.entity.is_none(), "no ship identity");
    assert!(gear.mesh.is_some(), "the gear is drawn");
    assert_eq!(gear.parent, Some(0));
    let entities = manifest
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Entity)
        .count();
    assert_eq!(entities, 3, "the ship, the tender and the snub craft");
    Ok(())
}
