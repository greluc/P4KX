// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Morph targets on a skinned piece: sparse `POSITION` deltas, names, weights and the checks that
//! leave out a target that does not fit.

use std::error::Error;

use p4kx_gltf::{GlbOptions, GlbSummary, MorphTarget, RigPiece, write_rigged_glb};
use p4kx_model::MaterialStatus;
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, TestModel, check_glb};

use crate::rig::{body, piece_model, skeleton_model};

type TestResult = Result<(), Box<dyn Error>>;

fn target(name: &str, moves: &[(u32, [f32; 3])]) -> MorphTarget {
    MorphTarget {
        name: name.to_owned(),
        vertices: moves.iter().map(|&(vertex, _)| vertex).collect(),
        deltas: moves.iter().map(|&(_, delta)| delta).collect(),
    }
}

fn write(targets: &[MorphTarget]) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let body = skeleton_model(&body())?;
    let piece = piece_model(&TestModel::two_bone_skin())?;
    let materials = MaterialStatus::Unnamed;
    let pieces = [RigPiece::new("head", &piece, &materials).with_morphs(targets)];
    let mut bytes = Vec::new();
    let summary = write_rigged_glb(&body.bones, &pieces, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

#[test]
fn a_skinned_mesh_gets_its_targets_as_sparse_position_deltas() -> TestResult {
    let targets = [
        target("smile", &[(2, [0.01, 0.02, 0.03]), (7, [-0.5, 0.0, 0.25])]),
        target("blink", &[(0, [0.0, 0.0, 0.001])]),
    ];
    let (glb, summary) = write(&targets)?;

    assert_eq!(summary.morph_targets, 2);
    let mesh = glb.document.meshes().next().ok_or("a mesh")?;
    assert_eq!(
        glb.target_names(mesh.index()),
        Some(vec!["smile".to_owned(), "blink".to_owned()])
    );
    assert_eq!(mesh.weights(), Some(&[0.0, 0.0][..]));
    let primitives: Vec<_> = mesh.primitives().collect();
    assert!(!primitives.is_empty());
    for primitive in &primitives {
        assert!(primitive.get(&Semantic::Joints(0)).is_some(), "skinned");
        assert_eq!(primitive.morph_targets().len(), 2);
    }
    let vertices = primitives[0]
        .get(&Semantic::Positions)
        .ok_or("POSITION")?
        .count();
    let smile = primitives[0]
        .morph_targets()
        .next()
        .and_then(|target| target.positions())
        .ok_or("a target")?;
    let dense = glb.floats(&smile);
    assert_eq!(dense.len(), vertices * 3);
    assert_eq!(&dense[6..9], &[0.01, 0.03, -0.02], "converted to Y up");
    assert_eq!(&dense[21..24], &[-0.5, 0.25, 0.0]);
    assert_eq!(dense.iter().filter(|value| **value != 0.0).count(), 5);
    let sparse = smile.sparse().ok_or("a sparse accessor")?;
    assert_eq!(sparse.count(), 2);
    assert!(smile.view().is_none(), "no dense base");
    Ok(())
}

#[test]
fn the_bounds_of_a_target_cover_the_zero_of_the_vertices_it_leaves() -> TestResult {
    let (glb, _) = write(&[target("up", &[(1, [0.0, 0.5, 0.0]), (2, [0.0, 0.75, 0.0])])])?;

    let mesh = glb.document.meshes().next().ok_or("a mesh")?;
    let positions = mesh
        .primitives()
        .next()
        .and_then(|primitive| primitive.morph_targets().next())
        .and_then(|target| target.positions())
        .ok_or("a target")?;
    let max = positions.max().ok_or("max")?;
    let min = positions.min().ok_or("min")?;
    assert_eq!(max, serde_values([0.0, 0.0, 0.0]));
    assert_eq!(min, serde_values([0.0, 0.0, -0.75]));
    Ok(())
}

fn serde_values(values: [f64; 3]) -> p4kx_testkit::gltf::json::Value {
    p4kx_testkit::gltf::json::Value::from(values.to_vec())
}

#[test]
fn targets_that_do_not_fit_the_mesh_are_left_out() -> TestResult {
    let targets = [
        target("empty", &[]),
        target("beyond", &[(24, [0.1, 0.0, 0.0])]),
        target("descending", &[(5, [0.1, 0.0, 0.0]), (3, [0.1, 0.0, 0.0])]),
        target("twice", &[(3, [0.1, 0.0, 0.0]), (3, [0.1, 0.0, 0.0])]),
        target("nan", &[(3, [f32::NAN, 0.0, 0.0])]),
        MorphTarget {
            name: "short".to_owned(),
            vertices: vec![1, 2],
            deltas: vec![[0.0; 3]],
        },
        target("fine", &[(3, [0.1, 0.0, 0.0])]),
    ];
    let (glb, summary) = write(&targets)?;

    assert_eq!(summary.morph_targets, 1);
    let mesh = glb.document.meshes().next().ok_or("a mesh")?;
    assert_eq!(
        glb.target_names(mesh.index()),
        Some(vec!["fine".to_owned()])
    );
    Ok(())
}

#[test]
fn a_piece_without_targets_has_no_target_names_or_weights() -> TestResult {
    let (glb, summary) = write(&[])?;

    assert_eq!(summary.morph_targets, 0);
    let mesh = glb.document.meshes().next().ok_or("a mesh")?;
    assert_eq!(glb.target_names(mesh.index()), None);
    assert_eq!(mesh.weights(), None);
    Ok(())
}

fn glb_bytes(targets: &[MorphTarget]) -> Result<Vec<u8>, Box<dyn Error>> {
    let body = skeleton_model(&body())?;
    let piece = piece_model(&TestModel::two_bone_skin())?;
    let materials = MaterialStatus::Unnamed;
    let pieces = [RigPiece::new("head", &piece, &materials).with_morphs(targets)];
    let mut bytes = Vec::new();
    write_rigged_glb(&body.bones, &pieces, &GlbOptions::default(), &mut bytes)?;
    Ok(bytes)
}

fn replace(bytes: &mut [u8], from: &[u8], to: &[u8]) -> TestResult {
    let at = bytes
        .windows(from.len())
        .position(|window| window == from)
        .ok_or("the bytes to corrupt")?;
    bytes[at..at + to.len()].copy_from_slice(to);
    Ok(())
}

#[test]
fn the_checker_rejects_broken_sparse_accessors_and_target_names() -> TestResult {
    let targets = [target("smile", &[(2, [0.01; 3]), (7, [0.02; 3])])];
    let good = glb_bytes(&targets)?;
    assert!(check_glb(&good).is_ok());

    let mut unsorted = good.clone();
    replace(&mut unsorted, &[2, 0, 7, 0], &[7, 0, 2, 0])?;
    assert!(check_glb(&unsorted).is_err_and(|error| error.contains("ascend")));

    let mut too_many = good.clone();
    replace(
        &mut too_many,
        br#""sparse":{"count":2"#,
        br#""sparse":{"count":9"#,
    )?;
    assert!(check_glb(&too_many).is_err());

    let mut renamed = good;
    replace(
        &mut renamed,
        br#""targetNames":["smile"]"#,
        br#""targetNames":["a","b"]"#,
    )?;
    assert!(check_glb(&renamed).is_err());
    Ok(())
}
