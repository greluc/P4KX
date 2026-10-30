// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A bone attachment of a character definition is an object node below its part.

use p4kx_blender::{Manifest, NodeKind, PackageInfo, write_package};
use p4kx_extraction::Archive;
use p4kx_testkit::{
    BONE_THRUSTER, bone_thruster_definition, bone_thruster_entries, bone_thruster_housing,
};

use crate::support::{TestResult, assemble, fixture, glb_options, never};

#[test]
fn an_attachment_is_an_object_node_with_a_mesh_below_its_part() -> TestResult {
    let definition =
        bone_thruster_definition(&bone_thruster_housing("tip", r"Objects\Test\housing.cgf"));
    let fixture = fixture(bone_thruster_entries(&definition))?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, BONE_THRUSTER)?;
    let models = p4kx_assembly::ArchiveModels::new(&archive);
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("thruster");

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
    let housing = manifest
        .nodes
        .iter()
        .find(|node| node.name == "Housing")
        .ok_or("the attachment node")?;
    assert_eq!(housing.kind, NodeKind::Object);
    assert_eq!(housing.path, "Housing");
    assert_eq!(housing.parent, Some(0));
    assert!(housing.entity.is_none(), "no Entity identity");
    assert!(housing.mesh.is_some(), "the housing is drawn");
    Ok(())
}
