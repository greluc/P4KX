// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Rounding of the weights over both influence sets and the opt-in limit of bones per vertex.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, write_glb};
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, TestModel, TestSkeleton, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

const WEIGHTS: [u8; 8] = [50, 50, 50, 40, 40, 30, 20, 20];

fn glb(options: &GlbOptions) -> Result<CheckedGlb, Box<dyn Error>> {
    let entry: Vec<(u16, u8)> = (0..8).zip(WEIGHTS).collect();
    let source = TestModel::cube()
        .with_skeleton(&TestSkeleton::chain(8))
        .with_bone_map(true, vec![entry; 24]);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("skin.skinm") => Some(source.cgfm()),
            _ => None,
        })
    };
    let model = Model::read(
        "skin.skin",
        &ChunkFile::parse(source.cgf())?,
        &mut companions,
    )?;
    let mut bytes = Vec::new();
    write_glb(&model, options, &mut bytes)?;
    Ok(check_glb(&bytes)?)
}

fn first_weights(glb: &CheckedGlb, set: u32) -> Result<Option<[u8; 4]>, Box<dyn Error>> {
    let primitive = glb
        .document
        .meshes()
        .next()
        .and_then(|mesh| mesh.primitives().next())
        .ok_or("a primitive")?;
    Ok(primitive
        .get(&Semantic::Weights(set))
        .and_then(|accessor| glb.bytes(&accessor).first_chunk::<4>().copied()))
}

#[test]
fn the_rounding_remainder_goes_to_the_strongest_bone_and_both_sets_add_up_to_255() -> TestResult {
    let glb = glb(&GlbOptions::default())?;

    let first = first_weights(&glb, 0)?.ok_or("set 0")?;
    let second = first_weights(&glb, 1)?.ok_or("set 1")?;
    assert_eq!(first, [44, 42, 42, 34], "set 0, the remainder of 2 on top");
    assert_eq!(second, [34, 25, 17, 17], "set 1");
    let total: u32 = first.iter().chain(&second).map(|&w| u32::from(w)).sum();
    assert_eq!(total, 255, "over both sets");
    Ok(())
}

#[test]
fn at_most_four_influences_keep_the_strongest_and_write_one_set() -> TestResult {
    let glb = glb(&GlbOptions::default().with_max_influences(4))?;

    assert_eq!(first_weights(&glb, 1)?, None, "no second set");
    let first = first_weights(&glb, 0)?.ok_or("set 0")?;
    assert_eq!(first, [68, 67, 67, 53], "the strongest four, rescaled");
    assert_eq!(first.iter().map(|&w| u32::from(w)).sum::<u32>(), 255);
    Ok(())
}

#[test]
fn a_limit_outside_one_to_eight_is_clamped() {
    assert_eq!(
        GlbOptions::default().with_max_influences(0).max_influences,
        1
    );
    assert_eq!(
        GlbOptions::default().with_max_influences(99).max_influences,
        8
    );
}
