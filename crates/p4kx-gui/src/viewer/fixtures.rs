// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::chunks::ChunkFile;
use p4kx_extraction::model::{Animation, CompanionError, CompanionPath, Model};
use p4kx_testkit::{TestAnimation, TestModel, TestSkeleton, TestTrack};

const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];
const BIND_TIP: [f32; 4] = [0.5, 0.5, 0.5, 0.5];

pub(super) fn skin_model() -> Model {
    let source = TestModel::two_bone_skin();
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("skin.skinm") => Some(source.cgfm()),
            _ => None,
        })
    };
    let file = ChunkFile::parse(source.cgf()).unwrap_or_else(|error| panic!("the skin: {error}"));
    Model::read("skin.skin", &file, &mut companions)
        .unwrap_or_else(|error| panic!("the skin: {error}"))
}

pub(super) fn swing(name: &str, last_frame: u16) -> Animation {
    let tip = TestSkeleton::controller_id(1);
    let animation = TestAnimation::new(name, 30)
        .track(TestTrack::new(tip).rotations(&[(0, BIND_TIP), (last_frame, QUARTER_TURN_Z)]));
    let file = ChunkFile::parse(animation.caf()).unwrap_or_else(|error| panic!("the caf: {error}"));
    Animation::read_caf(&format!("{name}.caf"), &file)
        .unwrap_or_else(|error| panic!("the animation: {error}"))
}
