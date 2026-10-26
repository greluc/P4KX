// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The head of a CHF character: shaped by its DNA, worn on the body of its variant, exported as
//! one skinned GLB, and what happens when the rig does not fit.

use std::error::Error;

use p4kx_assembly::character::{Dna, DnaBlend};
use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, ModelProvider, PartOrigin,
    Piece,
};
use p4kx_extraction::Archive;
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{
    HEAD_REGIONS, HEAD_VERTICES, MALE_HEAD_FOLDER, MALE_RIG, MALE_RIG_HASH, TestEntry, check_glb,
    head_entries, head_rig,
};

use crate::assembly_support::{TestResult, fixture};

pub(crate) fn head_path() -> String {
    format!("{MALE_HEAD_FOLDER}/{MALE_RIG}_head.skin")
}

pub(crate) fn dna_of_head_two_in_region_three() -> Dna {
    let blends = |value, head_id| vec![DnaBlend { value, head_id }];
    Dna {
        gender_hash: MALE_RIG_HASH,
        label: p4kx_character::Label,
        variant_hash: 0,
        blends_per_part: 1,
        header_unknown: 0,
        max_head_id: 2,
        face_parts: (0..HEAD_REGIONS)
            .map(|part| {
                if part == 3 {
                    blends(65535, 2)
                } else {
                    blends(0, 0)
                }
            })
            .collect(),
    }
}

fn build(entries: Vec<TestEntry>, dna: Dna) -> Result<Armour, Box<dyn Error>> {
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let piece = Piece::file("head", head_path(), None).shaped(dna);
    Ok(Armour::build(
        BodyVariant::Male,
        &[piece],
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn shaped_armour() -> Result<Armour, Box<dyn Error>> {
    build(
        head_entries(Some(head_rig(HEAD_VERTICES, MALE_RIG_HASH).build())),
        dna_of_head_two_in_region_three(),
    )
}

fn head_part(armour: &Armour) -> Result<&ArmourPart, Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .find(|part| part.name() == "head")
        .ok_or("the head part")?)
}

#[test]
fn the_vertices_of_the_head_move_by_the_morphs_of_the_dnas_heads() -> TestResult {
    let armour = shaped_armour()?;

    let fixture = fixture(head_entries(None))?;
    let archive = Archive::open(&fixture.path)?;
    let plain = ArchiveModels::new(&archive).model(&head_path(), 0)?;
    let before = &plain.model.lods[0].meshes[0];
    let after = &head_part(&armour)?.model().lods[0].meshes[0];

    assert!(armour.issues().is_empty(), "{:?}", armour.issues());
    assert_eq!(before.source, after.source);
    assert_eq!(
        after.source.first(),
        Some(&5),
        "the first triangle uses vertex 5"
    );
    for (index, &source) in after.source.iter().enumerate() {
        let expected = if source == 5 {
            [0.1, 0.1, 0.03]
        } else {
            [0.0; 3]
        };
        let pairs = after.positions[index].iter().zip(before.positions[index]);
        for (axis, ((now, was), want)) in pairs.zip(expected).enumerate() {
            let moved = now - was;
            assert!(
                (moved - want).abs() < 1e-5,
                "vertex {source} axis {axis}: moved {moved}, expected {want}"
            );
        }
    }
    Ok(())
}

fn bitangent(tangent: &[f32; 4], normal: &[f32; 3]) -> [f32; 3] {
    let [tx, ty, tz, w] = *tangent;
    let [nx, ny, nz] = *normal;
    [
        ny.mul_add(tz, -(nz * ty)) * w,
        nz.mul_add(tx, -(nx * tz)) * w,
        nx.mul_add(ty, -(ny * tx)) * w,
    ]
}

#[test]
fn the_shaped_tangents_follow_the_new_normals_and_keep_their_handedness() -> TestResult {
    let armour = shaped_armour()?;
    let fixture = fixture(head_entries(None))?;
    let archive = Archive::open(&fixture.path)?;
    let plain = ArchiveModels::new(&archive).model(&head_path(), 0)?;
    let before = &plain.model.lods[0].meshes[0];
    let after = &head_part(&armour)?.model().lods[0].meshes[0];
    let (old_normals, new_normals) = (
        before.normals.as_ref().ok_or("normals")?,
        after.normals.as_ref().ok_or("shaped normals")?,
    );
    let (old_tangents, new_tangents) = (
        before.tangents.as_ref().ok_or("tangents")?,
        after.tangents.as_ref().ok_or("shaped tangents")?,
    );

    let mut turned = 0;
    for (index, (((tangent, old), normal), old_normal)) in new_tangents
        .iter()
        .zip(old_tangents)
        .zip(new_normals)
        .zip(old_normals)
        .enumerate()
    {
        let kept: f32 = tangent.iter().zip(old).take(3).map(|(t, o)| t * o).sum();
        assert!(kept > 0.9, "vertex {index}: the tangent flipped: {kept}");
        let (new_side, old_side) = (bitangent(tangent, normal), bitangent(old, old_normal));
        let side: f32 = new_side.iter().zip(old_side).map(|(n, o)| n * o).sum();
        assert!(side > 0.9, "vertex {index}: the bitangent flipped: {side}");
        let along: f32 = tangent.iter().zip(normal).map(|(t, n)| t * n).sum();
        let length = tangent[..3]
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!(
            along.abs() < 1e-4,
            "vertex {index}: tangent not perpendicular: {along}"
        );
        assert!(
            (length - 1.0).abs() < 1e-3,
            "vertex {index}: length {length}"
        );
        assert_eq!(tangent[3], old[3], "vertex {index}: handedness");
        let shift: f32 = tangent.iter().zip(old).map(|(t, o)| (t - o).abs()).sum();
        turned += usize::from(shift > 1e-3);
    }
    assert!(turned > 0, "the tangents of the moved corner turned");
    let shifted = old_normals.iter().zip(new_normals).any(|(old, new)| {
        old.iter()
            .zip(new)
            .any(|(old, new)| (old - new).abs() > 1e-3)
    });
    assert!(shifted, "the normals of the moved corner turned");
    Ok(())
}

#[test]
fn the_shaped_normals_stay_unit_length() -> TestResult {
    let armour = shaped_armour()?;

    let normals = head_part(&armour)?.model().lods[0].meshes[0]
        .normals
        .as_ref()
        .ok_or("normals")?;

    for normal in normals {
        let length = normal.iter().map(|value| value * value).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-3, "{normal:?}");
    }
    Ok(())
}

#[test]
fn the_head_is_worn_on_the_body_and_exported_as_one_skinned_glb() -> TestResult {
    let armour = shaped_armour()?;
    let mut bytes = Vec::new();

    let summary = armour.write_glb(&GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes)?;

    let origins: Vec<(&str, PartOrigin)> = armour
        .parts()
        .iter()
        .map(|part| (part.name(), part.origin()))
        .collect();
    assert_eq!(
        origins,
        [("body", PartOrigin::Body), ("head", PartOrigin::Piece)]
    );
    assert_eq!(summary.skinned_meshes, 2);
    assert_eq!(
        glb.document.skins().count(),
        1,
        "one skin over the body skeleton"
    );
    let head = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("head") && node.mesh().is_some())
        .ok_or("the head node")?;
    assert!(head.skin().is_some(), "the head is skinned");
    Ok(())
}

fn only_issue(armour: &Armour) -> Result<&p4kx_assembly::Issue, Box<dyn Error>> {
    assert_eq!(armour.parts().len(), 1, "only the body is left");
    let issue = armour.issues().first().ok_or("an issue")?;
    assert_eq!(issue.kind().code(), "shape_failed", "the kind");
    assert!(issue.kind().skips(), "the head is skipped");
    Ok(issue)
}

#[test]
fn a_head_whose_dna_rig_is_missing_is_skipped_and_listed() -> TestResult {
    let armour = build(head_entries(None), dna_of_head_two_in_region_three())?;

    let issue = only_issue(&armour)?;

    assert!(
        issue.kind().to_string().contains("does not exist"),
        "{}",
        issue.kind()
    );
    Ok(())
}

#[test]
fn a_dna_for_another_rig_does_not_shape_the_head() -> TestResult {
    let armour = build(
        head_entries(Some(head_rig(HEAD_VERTICES, 0x1234_5678).build())),
        dna_of_head_two_in_region_three(),
    )?;

    let issue = only_issue(&armour)?;

    assert!(
        issue.kind().to_string().contains("another rig")
            || issue.kind().to_string().contains("is the rig"),
        "{}",
        issue.kind()
    );
    Ok(())
}

#[test]
fn a_rig_that_covers_fewer_vertices_than_the_head_uses_is_refused() -> TestResult {
    let armour = build(
        head_entries(Some(head_rig(6, MALE_RIG_HASH).build())),
        dna_of_head_two_in_region_three(),
    )?;

    let issue = only_issue(&armour)?;

    assert!(
        issue
            .kind()
            .to_string()
            .contains("covers 6 vertices, the skin has 24"),
        "{}",
        issue.kind()
    );
    Ok(())
}

#[test]
fn a_rig_that_covers_more_vertices_than_the_skin_has_is_refused() -> TestResult {
    let armour = build(
        head_entries(Some(head_rig(HEAD_VERTICES + 6, MALE_RIG_HASH).build())),
        dna_of_head_two_in_region_three(),
    )?;

    let issue = only_issue(&armour)?;

    let text = issue.kind().to_string();
    assert!(
        text.contains("covers 30 vertices, the skin has 24"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_rig_whose_moves_are_not_finite_is_refused() -> TestResult {
    let mut rig = head_rig(HEAD_VERTICES, MALE_RIG_HASH);
    rig.controls[16].scale = [f32::NAN; 3];

    let armour = build(
        head_entries(Some(rig.build())),
        dna_of_head_two_in_region_three(),
    )?;

    let issue = only_issue(&armour)?;
    assert!(
        issue.kind().to_string().contains("not finite"),
        "{}",
        issue.kind()
    );
    Ok(())
}

#[test]
fn a_dna_naming_a_head_the_rig_lacks_is_refused() -> TestResult {
    let mut dna = dna_of_head_two_in_region_three();
    dna.face_parts[3] = vec![DnaBlend {
        value: 100,
        head_id: 40,
    }];

    let armour = build(
        head_entries(Some(head_rig(HEAD_VERTICES, MALE_RIG_HASH).build())),
        dna,
    )?;

    let issue = only_issue(&armour)?;
    assert!(
        issue.kind().to_string().contains("head 40"),
        "{}",
        issue.kind()
    );
    Ok(())
}
