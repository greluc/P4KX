//! Skeleton files, levels of detail with a skeleton of their own and the helper bones of static
//! models written as GLB.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GlbSummary, write_glb};
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::{CheckedGlb, TestModel, TestSkeleton, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

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

fn glb(source: &TestModel) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = write_glb(&model(source)?, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

#[test]
fn a_skeleton_file_without_meshes_exports_its_bones_as_nodes() -> TestResult {
    let source = TestModel::cube()
        .helpers_only()
        .with_skeleton(&TestSkeleton::chain(3));
    let mut none = |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    let model = Model::read("skin.chr", &ChunkFile::parse(source.cgf())?, &mut none)?;
    let mut bytes = Vec::new();
    let summary = write_glb(&model, &GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes)?;

    assert_eq!(glb.document.skins().len(), 0, "no skin without a mesh");
    assert_eq!(glb.document.meshes().len(), 0, "no mesh");
    assert_eq!(
        (summary.bones, summary.skinned_meshes),
        (3, 0),
        "three bones"
    );
    let names: Vec<Option<&str>> = glb.document.nodes().map(|node| node.name()).collect();
    assert_eq!(
        names,
        [
            Some("cube"),
            Some("cube_mesh"),
            Some("bone_0"),
            Some("bone_1"),
            Some("bone_2")
        ],
        "the model's nodes, then the bones"
    );
    let bone = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("bone_1"))
        .ok_or("bone_1")?;
    let (translation, _, _) = bone.transform().decomposed();
    assert_eq!(translation, [1.0, 0.0, 0.0], "1 m along X from its parent");
    assert_eq!(bone.children().count(), 1, "bone_2 hangs below bone_1");
    Ok(())
}

#[test]
fn a_level_of_detail_is_exported_with_the_skeleton_of_its_own_file() -> TestResult {
    let level_one = TestModel::cube()
        .with_skeleton(&TestSkeleton::chain(6))
        .with_bone_map(false, vec![vec![(5, 255)]; 24]);
    let source = TestModel::two_bone_skin().with_lod(level_one);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("skin.skinm") => Some(source.cgfm()),
            CompanionPath::Sibling("skin_lod1.skinm") => source.lod_cgfm(1),
            CompanionPath::Sibling("skin_lod1.skin") => source.lod_cgf(1),
            _ => None,
        })
    };
    let model = Model::read(
        "skin.skin",
        &ChunkFile::parse(source.cgf())?,
        &mut companions,
    )?;

    for (lod, joints) in [(0, 2), (1, 6)] {
        let mut bytes = Vec::new();
        let summary = write_glb(&model, &GlbOptions::default().with_lod(lod), &mut bytes)?;
        let glb = check_glb(&bytes)?;
        let skin = glb.document.skins().next().ok_or("a skin")?;
        assert_eq!(skin.joints().count(), joints, "joints of LOD{lod}");
        assert_eq!(summary.bones, joints, "bone nodes of LOD{lod}");
    }
    Ok(())
}

#[test]
fn the_helper_bones_of_a_static_model_are_not_written_as_nodes() -> TestResult {
    let source = TestModel::cube().with_compact_skeleton(&TestSkeleton::chain(3));
    let (glb, summary) = glb(&source)?;

    assert_eq!(glb.document.nodes().len(), 2, "the cube's own nodes");
    assert_eq!(glb.document.skins().len(), 0, "no skin");
    assert_eq!(summary.bones, 0, "no bone nodes");
    Ok(())
}
