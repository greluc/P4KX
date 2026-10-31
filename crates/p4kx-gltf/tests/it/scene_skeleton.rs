// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A scene given a skeleton binds its skinned meshes to it and writes the bones once.

use p4kx_gltf::{GlbOptions, GlbSummary, Scene, SceneNode, ScenePart};
use p4kx_model::{MaterialStatus, Model, Transform};
use p4kx_testkit::{CheckedGlb, TestModel, check_glb};

use crate::rig::{body, piece_model, skeleton_model};
use crate::scene_support::{TestResult, read_cube, translation};

fn write(
    skeleton: Option<&Model>,
    parts: &[(&str, &Model)],
) -> Result<(CheckedGlb, GlbSummary), Box<dyn std::error::Error>> {
    let materials = MaterialStatus::Unnamed;
    let mut scene =
        skeleton.map_or_else(Scene::new, |model| Scene::new().with_skeleton(&model.bones));
    let root = scene.push(SceneNode::new("root", None, Transform::IDENTITY))?;
    for (name, model) in parts {
        let node = SceneNode::new(*name, Some(root), translation(0.0, 0.0, 5.0))
            .with_part(ScenePart::new(model, &materials));
        scene.push(node)?;
    }
    let mut bytes = Vec::new();
    let summary = p4kx_gltf::write_scene_glb(&scene, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

#[test]
fn skinned_meshes_share_one_skin_over_the_bones_of_the_skeleton() -> TestResult {
    let skeleton = skeleton_model(&body())?;
    let skin = piece_model(&TestModel::two_bone_skin())?;

    let (glb, summary) = write(Some(&skeleton), &[("body/skin", &skin)])?;

    assert_eq!(glb.document.skins().len(), 1);
    let joints: Vec<Option<&str>> = glb
        .document
        .skins()
        .next()
        .ok_or("a skin")?
        .joints()
        .map(|joint| joint.name())
        .collect();
    assert_eq!(joints, [Some("pelvis"), Some("TIP"), Some("root")]);
    assert_eq!((summary.bones, summary.skinned_meshes), (3, 1));
    Ok(())
}

#[test]
fn a_skinned_mesh_node_is_a_root_without_transform() -> TestResult {
    let skeleton = skeleton_model(&body())?;
    let skin = piece_model(&TestModel::two_bone_skin())?;

    let (glb, _) = write(Some(&skeleton), &[("body/skin", &skin)])?;

    let node = glb
        .document
        .nodes()
        .find(|node| node.skin().is_some())
        .ok_or("a skinned node")?;
    assert_eq!(node.transform().decomposed().0, [0.0, 0.0, 0.0]);
    let mut roots = glb.document.default_scene().ok_or("a scene")?.nodes();
    assert!(
        roots.any(|root| root.index() == node.index()),
        "a root of the scene"
    );
    Ok(())
}

#[test]
fn an_unskinned_part_keeps_its_place_below_its_wrapper() -> TestResult {
    let skeleton = skeleton_model(&body())?;
    let skin = piece_model(&TestModel::two_bone_skin())?;
    let cube = read_cube()?;

    let (glb, summary) = write(Some(&skeleton), &[("body/skin", &skin), ("prop", &cube)])?;

    assert_eq!(summary.skinned_meshes, 1);
    let wrapper = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("prop"))
        .ok_or("the prop wrapper")?;
    assert_eq!(wrapper.transform().decomposed().0, [0.0, 5.0, 0.0]);
    assert!(wrapper.children().len() > 0, "the model hangs below it");
    Ok(())
}

#[test]
fn a_scene_without_a_skinned_mesh_writes_no_bones() -> TestResult {
    let skeleton = skeleton_model(&body())?;
    let cube = read_cube()?;

    let (glb, summary) = write(Some(&skeleton), &[("prop", &cube)])?;

    assert_eq!(glb.document.skins().len(), 0);
    assert_eq!(summary.bones, 0);
    Ok(())
}

#[test]
fn a_scene_without_a_skeleton_stays_static() -> TestResult {
    let skin = piece_model(&TestModel::two_bone_skin())?;

    let (glb, summary) = write(None, &[("body/skin", &skin)])?;

    assert_eq!(glb.document.skins().len(), 0);
    assert_eq!((summary.bones, summary.skinned_meshes), (0, 0));
    Ok(())
}
