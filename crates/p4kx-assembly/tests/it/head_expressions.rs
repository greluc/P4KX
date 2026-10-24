// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The expression blendshapes of a head as morph targets: named by the skin's RigLogic chunk,
//! mixed by the Character's DNA, limited to a chosen set, and valid in the GLB.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, Expressions, IssueKind, Piece,
};
use p4kx_extraction::Archive;
use p4kx_gltf::GlbOptions;
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{HEAD_VERTICES, MALE_RIG_HASH, check_glb, head_entries, head_rig};

use crate::assembly_support::{TestResult, fixture};
use crate::head_build::{dna_of_head_two_in_region_three, head_path};

fn build(rig: Option<Vec<u8>>, expressions: Expressions) -> Result<Armour, Box<dyn Error>> {
    let fixture = fixture(head_entries(rig))?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let piece = Piece::file("head", head_path(), None)
        .shaped(dna_of_head_two_in_region_three())
        .with_expressions(expressions);
    Ok(Armour::build(
        BodyVariant::Male,
        &[piece],
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn armour(expressions: Expressions) -> Result<Armour, Box<dyn Error>> {
    build(
        Some(head_rig(HEAD_VERTICES, MALE_RIG_HASH).build()),
        expressions,
    )
}

fn head(armour: &Armour) -> Result<&ArmourPart, Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .find(|part| part.name() == "head")
        .ok_or("the head part")?)
}

fn close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "{actual:?} against {expected:?}"
        );
    }
}

fn mesh_vertex(part: &ArmourPart, stream: u32) -> Result<u32, Box<dyn Error>> {
    let position = part.model().lods[0].meshes[0]
        .source
        .iter()
        .position(|&source| source == stream)
        .ok_or("the stream vertex")?;
    Ok(u32::try_from(position)?)
}

#[test]
fn without_expressions_the_head_has_no_morph_targets() -> TestResult {
    let armour = armour(Expressions::None)?;

    assert!(head(&armour)?.morphs().is_empty());
    assert!(armour.issues().is_empty(), "{:?}", armour.issues());
    Ok(())
}

#[test]
fn every_blendshape_the_head_moves_becomes_a_named_target_mixed_by_the_dna() -> TestResult {
    let armour = armour(Expressions::All)?;
    let part = head(&armour)?;

    let names: Vec<&str> = part
        .morphs()
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["CTL_a", "CTL_b"],
        "the blendshapes no head moves are left out"
    );
    assert!(armour.issues().is_empty(), "{:?}", armour.issues());

    let first = &part.morphs()[0];
    assert_eq!(first.vertices, [mesh_vertex(part, 5)?]);
    close(first.deltas[0], [0.12 / 13.0 + 0.03 / 13.0, 0.0, 0.0]);
    let second = &part.morphs()[1];
    assert_eq!(second.vertices, [mesh_vertex(part, 6)?]);
    close(second.deltas[0], [0.0, 0.02 / 13.0, 0.0]);
    Ok(())
}

#[test]
fn the_targets_are_written_to_a_valid_glb_with_their_names() -> TestResult {
    let armour = armour(Expressions::All)?;
    let mut bytes = Vec::new();

    let summary = armour.write_glb(&GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes)?;

    assert_eq!(summary.morph_targets, 2);
    let node = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("head") && node.mesh().is_some())
        .ok_or("the head node")?;
    let mesh = node.mesh().ok_or("the head mesh")?;
    assert!(node.skin().is_some(), "skinned and morphed");
    assert_eq!(
        glb.target_names(mesh.index()),
        Some(vec!["CTL_a".to_owned(), "CTL_b".to_owned()])
    );
    let primitive = mesh.primitives().next().ok_or("a primitive")?;
    let target = primitive
        .morph_targets()
        .next()
        .and_then(|target| target.positions())
        .ok_or("a target")?;
    let dense = glb.floats(&target);
    let moved: Vec<f32> = dense.iter().copied().filter(|v| *v != 0.0).collect();
    assert_eq!(moved.len(), 1, "one vertex, one axis");
    assert!((moved[0] - 0.15 / 13.0).abs() < 1e-5);
    assert!(primitive.get(&Semantic::Joints(0)).is_some());
    Ok(())
}

#[test]
fn an_option_limits_the_targets_to_the_chosen_expressions() -> TestResult {
    let armour = armour(Expressions::Only(vec![
        "ctl_B".to_owned(),
        "nope".to_owned(),
    ]))?;

    let names: Vec<&str> = head(&armour)?
        .morphs()
        .iter()
        .map(|target| target.name.as_str())
        .collect();
    assert_eq!(names, ["CTL_b"]);
    let issue = armour.issues().first().ok_or("an issue")?;
    assert_eq!(issue.kind().code(), "unknown_expression");
    assert!(!issue.kind().skips());
    assert_eq!(armour.issues().len(), 1);
    Ok(())
}

#[test]
fn a_name_matches_without_the_control_prefix() -> TestResult {
    let armour = armour(Expressions::Only(vec!["a".to_owned()]))?;

    assert_eq!(head(&armour)?.morphs().len(), 1);
    assert_eq!(head(&armour)?.morphs()[0].name, "CTL_a");
    Ok(())
}

#[test]
fn a_rig_whose_blendshapes_do_not_match_the_names_keeps_the_head_without_targets() -> TestResult {
    let mut rig = head_rig(HEAD_VERTICES, MALE_RIG_HASH);
    rig.blendshapes = 5;

    let armour = build(Some(rig.build()), Expressions::All)?;

    assert!(head(&armour)?.morphs().is_empty());
    let issue = armour.issues().first().ok_or("an issue")?;
    assert!(
        matches!(issue.kind(), IssueKind::MorphsFailed { .. }),
        "{issue:?}"
    );
    assert!(!issue.kind().skips(), "the shaped head stays");
    Ok(())
}
