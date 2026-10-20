//! The levels of detail of a skin are weighted to the skeleton of their own `_lod<n>` file.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{CompanionError, CompanionPath, CorruptChunk, LodSelection, Model, ModelError};
use p4kx_testkit::{TestModel, TestSkeleton};

type TestResult = Result<(), Box<dyn Error>>;

fn read(model: &TestModel, with_lod_file: bool, lods: LodSelection) -> Result<Model, ModelError> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("skin.skinm") => Some(model.cgfm()),
            CompanionPath::Sibling("skin_lod1.skinm") => model.lod_cgfm(1),
            CompanionPath::Sibling("skin_lod1.skin") if with_lod_file => model.lod_cgf(1),
            _ => None,
        })
    };
    let file = ChunkFile::parse(model.cgf()).map_err(|source| ModelError::CompanionChunks {
        file: "skin.skin".to_owned(),
        source,
    })?;
    Model::read_with("skin.skin", &file, &mut companions, lods)
}

fn base() -> TestModel {
    TestModel::two_bone_skin()
}

fn level_one(skeleton: &TestSkeleton, joint: u16) -> TestModel {
    TestModel::cube()
        .with_skeleton(skeleton)
        .with_bone_map(false, vec![vec![(joint, 255)]; 24])
}

#[test]
fn a_level_with_its_own_skeleton_is_weighted_to_it() -> TestResult {
    let chain = TestSkeleton::chain(6);
    let skin = base().with_lod(level_one(&chain, 5));
    let model = read(&skin, true, LodSelection::All)?;

    let [lod0, lod1] = model.lods.as_slice() else {
        return Err("two levels".into());
    };
    assert_eq!(model.bones.len(), 2, "the model file has two bones");
    assert!(lod0.bones.is_none(), "level 0 uses the model's bones");
    assert_eq!(model.bones_of(lod0).len(), 2, "two bones at level 0");
    assert_eq!(
        lod1.bones.as_ref().map(Vec::len),
        Some(6),
        "level 1 has the six bones of its file"
    );
    assert_eq!(model.bones_of(lod1).len(), 6, "six bones at level 1");
    Ok(())
}

#[test]
fn a_level_with_the_same_skeleton_shares_the_models() -> TestResult {
    let skin = base().with_lod(level_one(&TestSkeleton::two_bones(), 1));
    let model = read(&skin, true, LodSelection::All)?;

    let lod1 = model.lods.get(1).ok_or("level 1")?;
    assert!(lod1.bones.is_none(), "equal skeletons are kept once");
    Ok(())
}

#[test]
fn a_level_alone_still_finds_its_skeleton() -> TestResult {
    let skin = base().with_lod(level_one(&TestSkeleton::chain(6), 5));
    let model = read(&skin, true, LodSelection::Only(1))?;

    let lod1 = model.lods.first().ok_or("level 1")?;
    assert_eq!(model.bones_of(lod1).len(), 6, "the bones of level 1");
    assert_eq!(model.bones.len(), 2, "the model's bones are read too");
    Ok(())
}

#[test]
fn a_bone_beyond_the_skeleton_of_the_level_is_corrupt() {
    let skin = base().with_lod(level_one(&TestSkeleton::chain(6), 6));

    let error = read(&skin, true, LodSelection::All).err();
    assert!(
        matches!(
            &error,
            Some(ModelError::Corrupt {
                source: CorruptChunk::JointRange {
                    joint: 6,
                    bones: 6,
                    ..
                },
                ..
            })
        ),
        "{error:?}"
    );
}

#[test]
fn a_level_without_a_file_of_its_own_is_checked_against_the_models_bones() {
    let skin = base().with_lod(level_one(&TestSkeleton::chain(6), 5));

    let error = read(&skin, false, LodSelection::All).err();
    assert!(
        matches!(
            &error,
            Some(ModelError::Corrupt {
                source: CorruptChunk::JointRange {
                    joint: 5,
                    bones: 2,
                    ..
                },
                ..
            })
        ),
        "{error:?}"
    );
}
