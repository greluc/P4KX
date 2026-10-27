// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A piece without a skinned mesh hangs below the bone it is mounted on.

use std::error::Error;

use p4kx_gltf::{GlbOptions, RigMount, RigPiece, write_rigged_glb};
use p4kx_model::{MaterialStatus, Model, Transform};
use p4kx_testkit::{CheckedGlb, TestModel, TestNode, check_glb};

use crate::rig::{body, piece_model, skeleton_model};

type TestResult = Result<(), Box<dyn Error>>;

fn node(name: &str, parent: Option<u16>, kind: u16) -> TestNode {
    TestNode {
        name: name.to_owned(),
        parent,
        local: Transform::IDENTITY.rows,
        kind,
    }
}

fn pack() -> Result<Model, Box<dyn Error>> {
    piece_model(
        &TestModel::cube().with_nodes(vec![node("pack", None, 0), node("pack_loc", Some(0), 3)]),
    )
}

const fn shift(y: f32, z: f32) -> Transform {
    let mut transform = Transform::IDENTITY;
    transform.rows[1][3] = y;
    transform.rows[2][3] = z;
    transform
}

fn write(mount: Option<RigMount>) -> Result<CheckedGlb, Box<dyn Error>> {
    let skeleton = skeleton_model(&body())?;
    let model = pack()?;
    let worn = piece_model(&TestModel::two_bone_skin())?;
    let materials = MaterialStatus::Unnamed;
    let pieces = [
        RigPiece::new("body", &worn, &materials),
        RigPiece::new("pack/test", &model, &materials).mounted(mount),
    ];
    let mut bytes = Vec::new();
    write_rigged_glb(&skeleton.bones, &pieces, &GlbOptions::default(), &mut bytes)?;
    Ok(check_glb(&bytes)?)
}

fn parent_name<'a>(glb: &'a CheckedGlb, node: &str) -> Option<&'a str> {
    glb.document
        .nodes()
        .find(|parent| parent.children().any(|child| child.name() == Some(node)))
        .and_then(|parent| parent.name())
}

fn translation(glb: &CheckedGlb, node: &str) -> Result<[f32; 3], Box<dyn Error>> {
    let found = glb
        .document
        .nodes()
        .find(|candidate| candidate.name() == Some(node))
        .ok_or_else(|| format!("no node {node}"))?;
    Ok(found.transform().decomposed().0)
}

#[test]
fn a_mounted_piece_hangs_below_its_bone_and_takes_the_mount_transform() -> TestResult {
    let glb = write(Some(RigMount::new(1, shift(0.2, 0.3))))?;

    assert_eq!(parent_name(&glb, "pack"), Some("TIP"), "the bone");
    let [x, y, z] = translation(&glb, "pack")?;
    assert!(
        (x.abs() + (y - 0.3).abs() + (z + 0.2).abs()) < 1e-6,
        "{x} {y} {z}"
    );
    assert_eq!(
        parent_name(&glb, "pack_loc"),
        Some("pack"),
        "its helper stays below it"
    );
    Ok(())
}

#[test]
fn a_piece_without_a_mount_stays_a_root() -> TestResult {
    let glb = write(None)?;

    assert_eq!(parent_name(&glb, "pack"), None);
    assert_eq!(translation(&glb, "pack")?, [0.0; 3]);
    Ok(())
}

#[test]
fn a_mount_on_a_bone_the_skeleton_lacks_leaves_the_piece_a_root() -> TestResult {
    let glb = write(Some(RigMount::new(7, shift(0.2, 0.3))))?;

    assert_eq!(parent_name(&glb, "pack"), None);
    Ok(())
}
