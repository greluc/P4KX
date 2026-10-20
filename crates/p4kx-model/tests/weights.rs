//! Reading the bone weights of the testkit skins through `p4kx-model`.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{
    CompanionError, CompanionPath, CorruptChunk, Influences, MAX_INFLUENCES, Mesh, Model,
    ModelError,
};
use p4kx_testkit::{TestModel, TestSkeleton};

type TestResult = Result<(), Box<dyn Error>>;

fn read(model: &TestModel) -> Result<Model, ModelError> {
    let cgfm = model.cgfm();
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("skin.skinm") => Some(cgfm.clone()),
            _ => None,
        })
    };
    let file = ChunkFile::parse(model.cgf()).map_err(|source| ModelError::CompanionChunks {
        file: "skin.skin".to_owned(),
        source,
    })?;
    Model::read("skin.skin", &file, &mut companions)
}

fn mesh(model: &Model) -> Result<&Mesh, Box<dyn Error>> {
    let lod = model.lods.first().ok_or("LOD0")?;
    let [mesh] = lod.meshes.as_slice() else {
        return Err("one mesh".into());
    };
    Ok(mesh)
}

fn influences(model: &Model) -> Result<&[Influences], Box<dyn Error>> {
    Ok(mesh(model)?.influences.as_deref().ok_or("influences")?)
}

fn pairs(influences: &Influences) -> Vec<(u16, u8)> {
    influences
        .joints
        .iter()
        .copied()
        .zip(influences.weights)
        .take_while(|&(_, weight)| weight > 0)
        .collect()
}

#[test]
fn every_vertex_of_a_skin_has_its_bone() -> TestResult {
    let skin = TestModel::two_bone_skin();
    let model = read(&skin)?;

    let mesh = mesh(&model)?;
    let all = mesh.influences.as_deref().ok_or("influences")?;
    assert_eq!(all.len(), mesh.positions.len(), "one entry per vertex");
    for (position, influences) in mesh.positions.iter().zip(all) {
        let bone = u16::from(position[2] > 2.0);
        assert_eq!(pairs(influences), [(bone, 255)], "{position:?}");
        assert_eq!(influences.count(), 1, "one bone");
        assert_eq!(influences.sum(), 255, "full weight");
    }
    Ok(())
}

#[test]
fn a_mesh_without_a_bone_map_is_not_skinned() -> TestResult {
    let model = read(&TestModel::cube().with_skeleton(&TestSkeleton::two_bones()))?;

    assert!(mesh(&model)?.influences.is_none(), "no influences");
    assert_eq!(model.bones.len(), 2, "the bones are still read");
    Ok(())
}

#[test]
fn influences_are_ordered_by_weight_with_a_bone_listed_twice_added_up() -> TestResult {
    let mut entries = vec![vec![(0, 255)]; 24];
    entries[0] = vec![(0, 0), (1, 64), (0, 191)];
    entries[1] = vec![(1, 100), (1, 155)];
    entries[2] = vec![(0, 100), (1, 100), (0, 55)];
    let model = read(&TestModel::two_bone_skin().with_bone_map(false, entries))?;

    let all = influences(&model)?;
    assert_eq!(pairs(&all[0]), [(0, 191), (1, 64)], "strongest first");
    assert_eq!(pairs(&all[1]), [(1, 255)], "two entries of one bone");
    assert_eq!(
        pairs(&all[2]),
        [(0, 155), (1, 100)],
        "added up, then sorted"
    );
    assert_eq!(all[0].joints[2..], [0; 6], "unused entries are bone 0");
    assert_eq!(all[0].weights[2..], [0; 6], "unused entries weigh nothing");
    Ok(())
}

#[test]
fn heads_have_eight_influences_per_vertex() -> TestResult {
    let skeleton = TestSkeleton::chain(8);
    let weights = [10_u8, 20, 30, 40, 50, 45, 35, 25];
    let entry: Vec<(u16, u8)> = (0..8).zip(weights).collect();
    let model = read(
        &TestModel::cube()
            .with_skeleton(&skeleton)
            .with_bone_map(true, vec![entry; 24]),
    )?;

    let all = influences(&model)?;
    let first = all.first().ok_or("a vertex")?;
    assert_eq!(first.count(), MAX_INFLUENCES, "eight bones");
    assert_eq!(first.joints, [4, 5, 3, 6, 2, 7, 1, 0], "ordered by weight");
    assert_eq!(first.weights, [50, 45, 40, 35, 30, 25, 20, 10], "weights");
    assert_eq!(first.sum(), 255, "sum");
    Ok(())
}

#[test]
fn a_bone_the_skeleton_lacks_is_a_corrupt_mesh() -> TestResult {
    let mut entries = vec![vec![(0, 255)]; 24];
    entries[5] = vec![(0, 100), (9, 155)];
    let error = read(&TestModel::two_bone_skin().with_bone_map(false, entries))
        .err()
        .ok_or("a vertex weighted to bone 9 of 2 is accepted")?;

    assert!(
        matches!(
            &error,
            ModelError::Corrupt {
                source: CorruptChunk::JointRange {
                    vertex: 5,
                    joint: 9,
                    bones: 2
                },
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_non_finite_texture_coordinate_reads_as_zero() -> TestResult {
    let skin = TestModel::two_bone_skin().with_uv(0, [f32::NAN, f32::INFINITY]);
    let model = read(&skin)?;

    let uvs = &mesh(&model)?.uvs;
    assert_eq!(
        uvs.first(),
        Some(&[0.0, 0.0]),
        "a NaN and an infinity read as 0"
    );
    assert_eq!(uvs.get(1), Some(&[1.0, 0.0]), "the others are as stored");
    Ok(())
}

#[test]
fn a_bone_without_weight_is_not_checked_against_the_skeleton() -> TestResult {
    let mut entries = vec![vec![(0, 255)]; 24];
    entries[5] = vec![(9, 0), (1, 255)];
    let model = read(&TestModel::two_bone_skin().with_bone_map(false, entries))?;

    assert_eq!(
        pairs(&influences(&model)?[5]),
        [(1, 255)],
        "bone 9 is dropped"
    );
    Ok(())
}

#[test]
fn weights_without_any_skeleton_are_a_corrupt_mesh() {
    let model = TestModel::cube().with_bone_map(false, vec![vec![(0, 255)]; 24]);

    assert!(
        matches!(
            read(&model),
            Err(ModelError::Corrupt {
                source: CorruptChunk::JointRange { .. },
                ..
            })
        ),
        "a skin needs its bones"
    );
}
