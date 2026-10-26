// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The GLB checker of the testkit finds the animations a writer could get wrong.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, write_glb_animated};
use p4kx_model::{Animation, CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::animation::Property;
use p4kx_testkit::{TestAnimation, TestModel, TestSkeleton, TestTrack, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

fn animated_glb() -> Result<Vec<u8>, Box<dyn Error>> {
    let source = TestModel::two_bone_skin();
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
    let wave = TestAnimation::new("wave.caf", 30).track(
        TestTrack::new(TestSkeleton::controller_id(1))
            .rotations(&[(0, [0.0, 0.0, 0.0, 1.0]), (30, [0.0, 0.0, 1.0, 0.0])])
            .positions(&[(0, [0.0; 3]), (30, [0.0, 0.0, 1.0])]),
    );
    let animation = Animation::read_caf("wave.caf", &ChunkFile::parse(wave.caf())?)?;
    let mut bytes = Vec::new();
    write_glb_animated(&model, &[animation], &GlbOptions::default(), &mut bytes)?;
    Ok(bytes)
}

fn binary_offset(bytes: &[u8]) -> Result<usize, Box<dyn Error>> {
    let json_len = u32::from_le_bytes(bytes.get(12..16).ok_or("JSON length")?.try_into()?);
    Ok(12 + 8 + json_len as usize + 8)
}

#[test]
fn the_checker_rejects_broken_animations() -> TestResult {
    let bytes = animated_glb()?;
    let checked = check_glb(&bytes)?;
    let start = binary_offset(&bytes)?;
    let animation = checked.document.animations().next().ok_or("an animation")?;
    let offset_of = |property: Property, input: bool| -> Result<usize, Box<dyn Error>> {
        let channel = animation
            .channels()
            .find(|channel| channel.target().property() == property)
            .ok_or("a channel")?;
        let sampler = channel.sampler();
        let accessor = if input {
            sampler.input()
        } else {
            sampler.output()
        };
        Ok(start + accessor.view().ok_or("a view")?.offset())
    };

    let mut backwards = bytes.clone();
    let times = offset_of(Property::Rotation, true)?;
    backwards
        .get_mut(times + 4..times + 8)
        .ok_or("the second time")?
        .copy_from_slice(&(-1.0_f32).to_le_bytes());
    assert!(
        check_glb(&backwards).is_err(),
        "key times that do not increase are found"
    );

    let mut stretched = bytes.clone();
    stretched
        .get_mut(times + 4..times + 8)
        .ok_or("the second time")?
        .copy_from_slice(&5.0_f32.to_le_bytes());
    assert!(
        check_glb(&stretched).is_err(),
        "a last time that is not the declared max is found"
    );

    let mut long = bytes.clone();
    let rotations = offset_of(Property::Rotation, false)?;
    long.get_mut(rotations + 28..rotations + 32)
        .ok_or("a w component")?
        .copy_from_slice(&3.0_f32.to_le_bytes());
    assert!(
        check_glb(&long).is_err(),
        "a rotation that is not of unit length is found"
    );

    let mut not_a_number = bytes;
    let translations = offset_of(Property::Translation, false)?;
    not_a_number
        .get_mut(translations..translations + 4)
        .ok_or("a component")?
        .copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(
        check_glb(&not_a_number).is_err(),
        "a translation that is not a number is found"
    );
    Ok(())
}

#[test]
fn the_checker_finds_a_first_time_that_is_not_the_declared_min() -> TestResult {
    let mut bytes = animated_glb()?;
    let start = binary_offset(&bytes)?;
    let checked = check_glb(&bytes)?;
    let animation = checked.document.animations().next().ok_or("an animation")?;
    let channel = animation.channels().next().ok_or("a channel")?;
    let times = start + channel.sampler().input().view().ok_or("a view")?.offset();

    bytes
        .get_mut(times..times + 4)
        .ok_or("the first time")?
        .copy_from_slice(&0.25_f32.to_le_bytes());

    let message = check_glb(&bytes).err().ok_or("an error")?;
    assert!(
        message.contains("min"),
        "the declared min is checked: {message}"
    );
    Ok(())
}
