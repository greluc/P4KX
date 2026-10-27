// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The model of a landing gear is posed at the end of the clip its skeleton's parameters name.

use crate::assembly_support::{TestResult, fixture};

use p4kx_assembly::{ArchiveModels, ModelProvider};
use p4kx_extraction::Archive;
use p4kx_extraction::model::Model;
use p4kx_testkit::{TestAnimation, TestDba, TestEntry, TestModel, TestSkeleton, TestTrack};

const CDF: &str = r#"<CharacterDefinition>
  <Model File="Objects\Gear\Gear_CHR.chr"/>
  <AttachmentList>
    <Attachment Type="CA_SKIN" AName="Gear" Binding="Objects\Gear\gear.skin"/>
  </AttachmentList>
</CharacterDefinition>"#;

const PARAMS: &str = r##"<Params><AnimationList>
  <Animation name="#filepath" path="Animations/Gear"/>
  <Animation name="$TracksDatabase" path="Animations/Gear/gear.dba"/>
  <Animation name="gear_compress" path="gear_compress.caf"/>
  <Animation name="gear_extend" path="gear_extend.caf"/>
</AnimationList></Params>"##;

const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

fn entries(params: &str) -> Vec<TestEntry> {
    let skin = TestModel::two_bone_skin();
    let tip = TestSkeleton::controller_id(1);
    let database = TestDba::new()
        .animation(
            TestAnimation::new("animations/gear/gear_extend.caf", 30)
                .track(TestTrack::new(tip).rotations(&[(0, IDENTITY), (30, QUARTER_TURN_Z)])),
        )
        .animation(
            TestAnimation::new("animations/gear/gear_compress.caf", 30)
                .track(TestTrack::new(tip).rotations(&[(0, IDENTITY), (30, IDENTITY)])),
        );
    vec![
        TestEntry::zstd("Data\\Objects\\Gear\\gear.cdf", CDF.as_bytes().to_vec()),
        TestEntry::zstd(
            "Data\\Objects\\Gear\\Gear_CHR.chrparams",
            params.as_bytes().to_vec(),
        ),
        TestEntry::zstd("Data\\Objects\\Gear\\gear.skin", skin.cgf()),
        TestEntry::zstd("Data\\Objects\\Gear\\gear.skinm", skin.cgfm()),
        TestEntry::zstd("Data\\Animations\\Gear\\gear.dba", database.bytes()),
    ]
}

/// The positions of the vertices weighted to the root alone and those weighted to the tip alone.
fn by_bone(model: &Model) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let mut root = Vec::new();
    let mut tip = Vec::new();
    for mesh in model.lods.iter().flat_map(|lod| &lod.meshes) {
        let influences = mesh.influences.as_deref().unwrap_or_default();
        for (position, influence) in mesh.positions.iter().zip(influences) {
            if influence.joints[0] == 0 {
                root.push(*position);
            } else {
                tip.push(*position);
            }
        }
    }
    (root, tip)
}

fn moved(a: &[[f32; 3]], b: &[[f32; 3]]) -> bool {
    a.iter()
        .zip(b)
        .any(|(a, b)| a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-2))
}

#[test]
fn the_extend_clip_moves_the_vertices_of_the_bones_it_keys_and_nothing_else() -> TestResult {
    let fixture = fixture(entries(PARAMS))?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let read = models.model("Objects/Gear/gear.cdf", 0)?;

    let posed = models
        .deployed("Objects/Gear/gear.cdf", &read.model)?
        .ok_or("a deploy clip is found")?;

    let (root, tip) = by_bone(&read.model);
    let (posed_root, posed_tip) = by_bone(&posed);
    assert!(!root.is_empty() && !tip.is_empty());
    assert!(!moved(&root, &posed_root), "the root keeps its vertices");
    assert!(moved(&tip, &posed_tip), "the tip moved");
    assert!(posed_tip.iter().flatten().all(|value| value.is_finite()));
    Ok(())
}

#[test]
fn without_a_clip_that_extends_or_deploys_there_is_no_pose() -> TestResult {
    let params = PARAMS.replace("gear_extend", "gear_retract");
    let fixture = fixture(entries(&params))?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let read = models.model("Objects/Gear/gear.cdf", 0)?;

    let posed = models.deployed("Objects/Gear/gear.cdf", &read.model)?;

    assert!(posed.is_none());
    Ok(())
}

#[test]
fn a_model_that_is_not_a_character_definition_is_not_posed() -> TestResult {
    let fixture = fixture(entries(PARAMS))?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let read = models.model("Objects/Gear/gear.skin", 0)?;

    assert!(
        models
            .deployed("Objects/Gear/gear.skin", &read.model)?
            .is_none()
    );
    Ok(())
}
