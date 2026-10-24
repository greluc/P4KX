// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Pieces worn on a shared skeleton, written as one GLB with one skin.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GlbSummary, RigPiece, write_rigged_glb};
use p4kx_model::{CompanionError, CompanionPath, MaterialStatus, Model};
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, TestBone, TestModel, TestSkeleton, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

fn bone(name: &str, parent: Option<u16>, z: f32) -> TestBone {
    TestBone {
        name: name.to_owned(),
        parent,
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0, z],
    }
}

fn skeleton_model(skeleton: &TestSkeleton) -> Result<Model, Box<dyn Error>> {
    let source = TestModel::cube().helpers_only().with_skeleton(skeleton);
    let mut none = |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    Ok(Model::read(
        "body.chr",
        &ChunkFile::parse(source.cgf())?,
        &mut none,
    )?)
}

fn piece_model(source: &TestModel) -> Result<Model, Box<dyn Error>> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("piece.skinm") => Some(source.cgfm()),
            _ => None,
        })
    };
    Ok(Model::read(
        "piece.skin",
        &ChunkFile::parse(source.cgf())?,
        &mut companions,
    )?)
}

fn write(
    body: &Model,
    pieces: &[(&str, &Model)],
) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let materials = MaterialStatus::Unnamed;
    let pieces: Vec<RigPiece<'_>> = pieces
        .iter()
        .map(|(name, model)| RigPiece::new(name, model, &materials))
        .collect();
    let mut bytes = Vec::new();
    let summary = write_rigged_glb(&body.bones, &pieces, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

fn joints_of(glb: &CheckedGlb, node: &str) -> Result<Vec<u16>, Box<dyn Error>> {
    let mesh = glb
        .document
        .nodes()
        .find(|candidate| candidate.name() == Some(node))
        .and_then(|found| found.mesh())
        .ok_or("the piece's mesh")?;
    let accessor = mesh
        .primitives()
        .next()
        .and_then(|primitive| primitive.get(&Semantic::Joints(0)))
        .ok_or("joints")?;
    Ok(glb
        .bytes(&accessor)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect())
}

fn body() -> TestSkeleton {
    TestSkeleton::new(vec![
        bone("pelvis", None, 0.0),
        bone("TIP", Some(0), 1.0),
        bone("root", Some(1), 1.0),
    ])
}

#[test]
fn two_pieces_share_one_skin_over_the_body_skeleton() -> TestResult {
    let body = skeleton_model(&body())?;
    let first = piece_model(&TestModel::two_bone_skin())?;
    let second = piece_model(&TestModel::two_bone_skin())?;
    let (glb, summary) = write(&body, &[("torso/a", &first), ("legs/b", &second)])?;

    assert_eq!(glb.document.skins().len(), 1, "one skin for both pieces");
    let skin = glb.document.skins().next().ok_or("a skin")?;
    let names: Vec<Option<&str>> = skin.joints().map(|joint| joint.name()).collect();
    assert_eq!(names, [Some("pelvis"), Some("TIP"), Some("root")]);
    assert_eq!(
        (summary.parts, summary.bones, summary.skinned_meshes),
        (2, 3, 2)
    );
    for name in ["torso/a", "legs/b"] {
        let node = glb
            .document
            .nodes()
            .find(|candidate| candidate.name() == Some(name))
            .ok_or("a piece node")?;
        assert_eq!(node.skin().map(|skin| skin.index()), Some(0), "{name}");
    }
    Ok(())
}

#[test]
fn the_bones_of_a_piece_are_matched_to_the_skeleton_by_name() -> TestResult {
    let body = skeleton_model(&body())?;
    let piece = piece_model(&TestModel::two_bone_skin())?;
    let (glb, _) = write(&body, &[("torso/a", &piece)])?;

    let joints = joints_of(&glb, "torso/a")?;
    let mut used: Vec<u16> = joints
        .chunks(4)
        .filter_map(|set| set.first().copied())
        .collect();
    used.sort_unstable();
    used.dedup();
    assert_eq!(
        used,
        [1, 2],
        "root became joint 2 and tip, written TIP in the skeleton, joint 1"
    );
    Ok(())
}

#[test]
fn a_bone_the_skeleton_lacks_takes_its_nearest_ancestor() -> TestResult {
    let skeleton = TestSkeleton::new(vec![bone("pelvis", None, 0.0), bone("root", Some(0), 1.0)]);
    let body = skeleton_model(&skeleton)?;
    let piece = piece_model(&TestModel::two_bone_skin())?;
    let (glb, _) = write(&body, &[("torso/a", &piece)])?;

    let joints = joints_of(&glb, "torso/a")?;
    assert!(
        joints.chunks(4).all(|set| set.first() == Some(&1)),
        "root is joint 1 and the missing tip follows it"
    );
    Ok(())
}
