//! The GLB checker of the testkit finds the skins a writer could get wrong.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, write_glb};
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::{Primitive, Semantic};
use p4kx_testkit::{CheckedGlb, TestModel, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

fn skin_glb() -> Result<Vec<u8>, Box<dyn Error>> {
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
    let mut bytes = Vec::new();
    write_glb(&model, &GlbOptions::default(), &mut bytes)?;
    Ok(bytes)
}

fn primitive(glb: &CheckedGlb) -> Result<Primitive<'_>, Box<dyn Error>> {
    Ok(glb
        .document
        .meshes()
        .next()
        .and_then(|mesh| mesh.primitives().next())
        .ok_or("a primitive")?)
}

fn binary_offset(bytes: &[u8]) -> Result<usize, Box<dyn Error>> {
    let json_len = u32::from_le_bytes(bytes.get(12..16).ok_or("JSON length")?.try_into()?);
    Ok(12 + 8 + json_len as usize + 8)
}

#[test]
fn the_checker_rejects_broken_skins() -> TestResult {
    let bytes = skin_glb()?;
    let checked = check_glb(&bytes)?;
    let start = binary_offset(&bytes)?;
    let view = |semantic: Semantic| -> Result<usize, Box<dyn Error>> {
        let accessor = primitive(&checked)?.get(&semantic).ok_or("an accessor")?;
        Ok(start + accessor.view().ok_or("a view")?.offset())
    };

    let needle = b"\"joints\":[1,2]";
    let at = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .ok_or("the joint list")?;
    let mut twice = bytes.clone();
    twice
        .get_mut(at..at + needle.len())
        .ok_or("the joint list")?
        .copy_from_slice(b"\"joints\":[1,1]");
    assert!(check_glb(&twice).is_err(), "a joint listed twice is found");

    let mut out_of_range = bytes.clone();
    let joints = view(Semantic::Joints(0))?;
    out_of_range
        .get_mut(joints..joints + 2)
        .ok_or("joint bytes")?
        .copy_from_slice(&7_u16.to_le_bytes());
    assert!(
        check_glb(&out_of_range).is_err(),
        "a joint the skin lacks is found"
    );

    let mut light = bytes.clone();
    let weights = view(Semantic::Weights(0))?;
    *light.get_mut(weights).ok_or("weight byte")? = 100;
    assert!(
        check_glb(&light).is_err(),
        "weights that do not add up to 1 are found"
    );

    let mut weighted_zero = bytes;
    *weighted_zero.get_mut(joints + 2).ok_or("joint byte")? = 1;
    assert!(
        check_glb(&weighted_zero).is_err(),
        "a joint other than 0 with a weight of 0 is found"
    );
    Ok(())
}
