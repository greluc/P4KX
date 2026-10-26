// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Skeletons and skinned meshes written as GLB: joints, inverse bind matrices, weights.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GlbSummary, write_glb};
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, TestModel, TestSkeleton, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

pub(crate) type Matrix = [[f32; 4]; 4];

fn model(source: &TestModel) -> Result<Model, Box<dyn Error>> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("skin.skinm") => Some(source.cgfm()),
            _ => None,
        })
    };
    Ok(Model::read(
        "skin.skin",
        &ChunkFile::parse(source.cgf())?,
        &mut companions,
    )?)
}

pub(crate) fn glb(source: &TestModel) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = write_glb(&model(source)?, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

fn multiply(left: &Matrix, right: &Matrix) -> Matrix {
    right.map(|column| {
        let mut out = [0.0_f32; 4];
        for (factor, left_column) in column.iter().zip(left) {
            for (slot, value) in out.iter_mut().zip(left_column) {
                *slot = factor.mul_add(*value, *slot);
            }
        }
        out
    })
}

pub(crate) fn global(glb: &CheckedGlb, node: usize) -> Matrix {
    let nodes: Vec<_> = glb.document.nodes().collect();
    let mut parents = vec![None; nodes.len()];
    for parent in &nodes {
        for child in parent.children() {
            if let Some(slot) = parents.get_mut(child.index()) {
                *slot = Some(parent.index());
            }
        }
    }
    let mut chain = vec![node];
    while let Some(parent) = chain
        .last()
        .and_then(|&last| parents.get(last).copied().flatten())
    {
        chain.push(parent);
    }
    chain.iter().rev().fold(
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        |sum, &index| {
            nodes
                .get(index)
                .map_or(sum, |node| multiply(&sum, &node.transform().matrix()))
        },
    )
}

fn inverse_binds(glb: &CheckedGlb) -> Result<Vec<Matrix>, Box<dyn Error>> {
    let skin = glb.document.skins().next().ok_or("a skin")?;
    let accessor = skin
        .inverse_bind_matrices()
        .ok_or("inverse bind matrices")?;
    Ok(glb
        .floats(&accessor)
        .as_chunks::<16>()
        .0
        .iter()
        .map(|flat| {
            let columns = flat.as_chunks::<4>().0;
            std::array::from_fn(|column| columns.get(column).copied().unwrap_or_default())
        })
        .collect())
}

fn assert_identity(matrix: &Matrix, what: &str) {
    for (column, values) in matrix.iter().enumerate() {
        for (row, value) in values.iter().enumerate() {
            let expected = f32::from(u8::from(column == row));
            assert!(
                (value - expected).abs() < 1e-4,
                "{what}: {matrix:?} is no identity"
            );
        }
    }
}

fn primitive(glb: &CheckedGlb) -> Result<p4kx_testkit::gltf::Primitive<'_>, Box<dyn Error>> {
    Ok(glb
        .document
        .meshes()
        .next()
        .and_then(|mesh| mesh.primitives().next())
        .ok_or("a primitive")?)
}

fn joint_bytes(glb: &CheckedGlb, set: u32) -> Result<Vec<u16>, Box<dyn Error>> {
    let accessor = primitive(glb)?
        .get(&Semantic::Joints(set))
        .ok_or("joints")?;
    Ok(glb
        .bytes(&accessor)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect())
}

fn weight_bytes(glb: &CheckedGlb, set: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let accessor = primitive(glb)?
        .get(&Semantic::Weights(set))
        .ok_or("weights")?;
    Ok(glb.bytes(&accessor))
}

#[test]
fn a_skin_is_written_with_its_joints_weights_and_inverse_bind_matrices() -> TestResult {
    let source = TestModel::two_bone_skin();
    let (glb, summary) = glb(&source)?;

    let skin = glb.document.skins().next().ok_or("a skin")?;
    assert_eq!(glb.document.skins().len(), 1, "one skin");
    assert_eq!(skin.joints().count(), 2, "two joints");
    let names: Vec<Option<&str>> = skin.joints().map(|joint| joint.name()).collect();
    assert_eq!(names, [Some("root"), Some("tip")], "joint names");
    assert_eq!(
        skin.skeleton().map(|node| node.index()),
        Some(1),
        "the skeleton root is the root bone"
    );
    assert_eq!(
        (summary.bones, summary.skinned_meshes, summary.nodes),
        (2, 1, 3),
        "two bone nodes, one skinned mesh, three nodes"
    );

    let joints = joint_bytes(&glb, 0)?;
    let weights = weight_bytes(&glb, 0)?;
    assert_eq!(joints.len(), 24 * 4, "four joints per vertex");
    assert_eq!(weights.len(), 24 * 4, "four weights per vertex");
    let positions = primitive(&glb)?
        .get(&Semantic::Positions)
        .ok_or("positions")?;
    for (vertex, position) in glb.floats(&positions).as_chunks::<3>().0.iter().enumerate() {
        let tip = u16::from(position[1] > 2.0);
        assert_eq!(
            (joints[vertex * 4], weights[vertex * 4]),
            (tip, 255),
            "vertex {vertex} at {position:?}"
        );
        assert_eq!(
            weights[vertex * 4 + 1..vertex * 4 + 4],
            [0; 3],
            "no other weight"
        );
    }
    assert!(
        primitive(&glb)?.get(&Semantic::Joints(1)).is_none(),
        "four influences fit in the first set"
    );
    Ok(())
}

#[test]
fn the_skeleton_is_in_y_up_metres_and_matches_the_inverse_bind_matrices() -> TestResult {
    let skeleton = TestSkeleton::two_bones();
    let (glb, _) = glb(&TestModel::two_bone_skin())?;

    let skin = glb.document.skins().next().ok_or("a skin")?;
    let binds = inverse_binds(&glb)?;
    for (joint, bind) in skin.joints().zip(&binds) {
        let at = multiply(&global(&glb, joint.index()), bind);
        assert_identity(&at, "the bind pose of a joint");
    }
    let tip = skin.joints().nth(1).ok_or("the tip")?;
    let [_, _, _, [x, y, z, _]] = global(&glb, tip.index());
    assert!(
        x.abs() < 1e-5 && (y - 2.0).abs() < 1e-5 && z.abs() < 1e-5,
        "the tip stands 2 m up: {x} {y} {z}"
    );
    let world = skeleton.world();
    assert_eq!(world.len(), 2, "two world poses");
    Ok(())
}

#[test]
fn a_chain_of_turned_bones_keeps_its_bind_pose_through_the_basis_change() -> TestResult {
    let skeleton = TestSkeleton::chain(6);
    let entries = vec![vec![(0, 100), (5, 155)]; 24];
    let source = TestModel::cube()
        .with_skeleton(&skeleton)
        .with_bone_map(false, entries);
    let (glb, summary) = glb(&source)?;

    assert_eq!(summary.bones, 6, "six bone nodes");
    let skin = glb.document.skins().next().ok_or("a skin")?;
    for (joint, bind) in skin.joints().zip(inverse_binds(&glb)?) {
        let at = multiply(&global(&glb, joint.index()), &bind);
        assert_identity(&at, "the bind pose of a joint");
    }
    let last = skin.joints().last().ok_or("the last joint")?;
    let [_, _, _, [x, y, z, _]] = global(&glb, last.index());
    let world = skeleton.world();
    let expected = world.last().ok_or("world pose")?.1;
    assert!(
        (x - expected[0]).abs() < 1e-4
            && (y - expected[2]).abs() < 1e-4
            && (z + expected[1]).abs() < 1e-4,
        "the last bone is at {expected:?} in Z up, {x} {y} {z} in Y up"
    );
    Ok(())
}

#[test]
fn the_skinned_mesh_is_a_root_without_transform_even_below_a_moved_parent() -> TestResult {
    let source = TestModel::cube()
        .with_skeleton(&TestSkeleton::two_bones())
        .with_bone_map(false, vec![vec![(1, 255)]; 24]);
    let (glb, _) = glb(&source)?;

    let mesh_node = glb
        .document
        .nodes()
        .find(|node| node.skin().is_some())
        .ok_or("a node with a skin")?;
    assert_eq!(mesh_node.name(), Some("cube_mesh"), "the geometry node");
    let (translation, rotation, scale) = mesh_node.transform().decomposed();
    assert_eq!(translation, [0.0; 3], "the 10 m offset is not applied");
    assert_eq!(
        (rotation, scale),
        ([0.0, 0.0, 0.0, 1.0], [1.0; 3]),
        "no turn"
    );
    let scene = glb.document.scenes().next().ok_or("a scene")?;
    assert!(
        scene.nodes().any(|node| node.index() == mesh_node.index()),
        "it is a root of the scene"
    );
    let cube = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("cube"))
        .ok_or("the cube node")?;
    assert_eq!(
        cube.children().count(),
        0,
        "it no longer has the mesh node below"
    );
    Ok(())
}

#[test]
fn eight_influences_per_vertex_use_a_second_set() -> TestResult {
    let weights = [10_u8, 20, 30, 40, 50, 45, 35, 25];
    let entry: Vec<(u16, u8)> = (0..8).zip(weights).collect();
    let source = TestModel::cube()
        .with_skeleton(&TestSkeleton::chain(8))
        .with_bone_map(true, vec![entry; 24]);
    let (glb, _) = glb(&source)?;

    assert_eq!(joint_bytes(&glb, 0)?[..4], [4, 5, 3, 6], "strongest four");
    assert_eq!(joint_bytes(&glb, 1)?[..4], [2, 7, 1, 0], "the rest");
    assert_eq!(
        weight_bytes(&glb, 0)?[..4],
        [50, 45, 40, 35],
        "weights of set 0"
    );
    assert_eq!(
        weight_bytes(&glb, 1)?[..4],
        [30, 25, 20, 10],
        "weights of set 1"
    );
    Ok(())
}

#[test]
fn a_wide_stream_with_at_most_four_influences_needs_one_set() -> TestResult {
    let source = TestModel::cube()
        .with_skeleton(&TestSkeleton::chain(8))
        .with_bone_map(true, vec![vec![(7, 200), (3, 55)]; 24]);
    let (glb, _) = glb(&source)?;

    assert!(
        primitive(&glb)?.get(&Semantic::Joints(1)).is_none(),
        "no second set"
    );
    assert_eq!(joint_bytes(&glb, 0)?[..4], [7, 3, 0, 0], "bones");
    assert_eq!(weight_bytes(&glb, 0)?[..4], [200, 55, 0, 0], "weights");
    Ok(())
}

#[test]
fn weights_are_normalised_and_an_unweighted_vertex_is_bound_to_the_first_bone() -> TestResult {
    let mut entries = vec![vec![(1, 100), (0, 50)]; 24];
    entries[3] = vec![(1, 0)];
    entries[4] = vec![(1, 255)];
    let source = TestModel::cube()
        .with_skeleton(&TestSkeleton::two_bones())
        .with_bone_map(false, entries);
    let (glb, _) = glb(&source)?;

    let joints = joint_bytes(&glb, 0)?;
    let weights = weight_bytes(&glb, 0)?;
    let vertex = |index: usize| {
        (
            &joints[index * 4..index * 4 + 4],
            &weights[index * 4..index * 4 + 4],
        )
    };
    assert_eq!(
        vertex(0),
        ([1, 0, 0, 0].as_slice(), [170, 85, 0, 0].as_slice()),
        "scaled to 255"
    );
    assert_eq!(
        vertex(3),
        ([0, 0, 0, 0].as_slice(), [255, 0, 0, 0].as_slice()),
        "bone 0"
    );
    assert_eq!(
        vertex(4),
        ([1, 0, 0, 0].as_slice(), [255, 0, 0, 0].as_slice()),
        "as stored"
    );
    Ok(())
}
