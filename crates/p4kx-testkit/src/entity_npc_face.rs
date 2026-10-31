// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_armour::{armour_male_skeleton, skeleton_entry};
use crate::entity_head::{HEAD_VERTICES, head_skin_on};
use crate::entity_npc::{NPC_HEAD_FOLDER, npc_entries};
use crate::{
    BSHAPES_GPU_CHUNK, TestBone, TestEntry, TestLogicDeltas, TestLogicRecord, TestRigLogic,
    TestSkeleton, TestSkinBlendshapes, TestSkinEntry,
};

/// The `.RigLogic` file of the head of [`NPC_CLASS`](crate::NPC_CLASS) below `Data`, with `/` as
/// separator: named like its own head skin without `_head.skin`.
pub const NPC_RIG_LOGIC: &str = "Objects/Characters/RigLogic/npc_test_t1.RigLogic";
/// The skeleton file of the head, beside its skin, below `Data` with `\` as separator.
pub const NPC_SKELETON_FILE: &str =
    "Data\\Objects\\Characters\\Human\\heads\\male\\pu\\npc_test\\npc_test_t1_head_skel.chr";
/// The facial bone of the head, below `head`, which its rig logic moves.
pub const NPC_FACIAL_BONE: &str = "facial_nose";
/// How far along x the skeleton file puts [`NPC_FACIAL_BONE`] from where the head skin has it, in
/// metres.
pub const NPC_SKELETON_SHIFT: f32 = 0.02;

/// The index of [`NPC_FACIAL_BONE`] in the skeletons of the head.
const FACIAL_BONE: u16 = 3;

/// The skeleton of the head skin: the male body's and [`NPC_FACIAL_BONE`] below its `head`, shifted
/// along x by `shift` metres.
fn skeleton(shift: f32) -> TestSkeleton {
    let mut bones = armour_male_skeleton().bones().to_vec();
    bones.push(TestBone {
        name: NPC_FACIAL_BONE.to_owned(),
        parent: Some(2),
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [shift, 0.1, 0.0],
    });
    TestSkeleton::new(bones)
}

/// The rig logic of the head of the story character.
///
/// It has the controls of [`TestRigLogic::small`] and [`NPC_FACIAL_BONE`] as its one joint, which
/// blendshape 1 moves by 0.01 m along x and blendshape 2 turns by 0.5 rad about y.
#[must_use]
pub fn npc_face_logic() -> TestRigLogic {
    let record = |output, blendshape, coefficient| TestLogicRecord {
        output,
        entries: vec![(blendshape, coefficient)],
    };
    TestRigLogic {
        joints: vec![TestSkeleton::controller_id(usize::from(FACIAL_BONE))],
        deltas: Some(TestLogicDeltas {
            main: vec![record(0, 1, 10_000)],
            secondary: vec![record(1, 2, 500_000)],
            ..TestLogicDeltas::small()
        }),
        ..TestRigLogic::small()
    }
}

/// The blendshapes in the geometry of the head of the story character.
///
/// Blendshape 1 moves the stream vertex 5 by 0.05 m along x, blendshape 2 moves the stream vertex
/// 6 by 0.05 m along y.
#[must_use]
pub fn npc_face_shapes() -> TestSkinBlendshapes {
    let entry = |vertex, blendshape, codes| TestSkinEntry {
        vertex,
        blendshape,
        codes,
    };
    TestSkinBlendshapes {
        vertices: HEAD_VERTICES,
        ranges: vec![([0.001; 3], [-0.1; 3]); npc_face_logic().blendshapes()],
        entries: vec![entry(5, 1, [150, 100, 100]), entry(6, 2, [100, 150, 100])],
    }
}

/// Returns the Entries of [`npc_entries`] with the face of the character's own head.
///
/// The head skin has [`NPC_FACIAL_BONE`] and the geometry file the `BShapesGpu` chunk `shapes`;
/// the `.RigLogic` file `rig_logic` is at [`NPC_RIG_LOGIC`], and the skeleton file
/// [`NPC_SKELETON_FILE`] has the bone [`NPC_SKELETON_SHIFT`] away from where the skin has it.
#[must_use]
pub fn npc_face_entries_with(rig_logic: &[u8], shapes: &[u8]) -> Vec<TestEntry> {
    let stem = format!(
        "Data\\{}\\npc_test_t1_head",
        NPC_HEAD_FOLDER.replace('/', "\\")
    );
    let replaced = [format!("{stem}.skin"), format!("{stem}.skinm")];
    let mut entries: Vec<TestEntry> = npc_entries()
        .into_iter()
        .filter(|entry| {
            !replaced
                .iter()
                .any(|path| entry.path().eq_ignore_ascii_case(path))
        })
        .collect();
    let model = head_skin_on(&skeleton(0.0), FACIAL_BONE, &npc_face_logic())
        .with_geometry_chunk(BSHAPES_GPU_CHUNK, shapes);
    entries.push(TestEntry::stored(&replaced[0], model.cgf()));
    entries.push(TestEntry::stored(&replaced[1], model.cgfm()));
    entries.push(TestEntry::stored(
        format!("Data\\{}", NPC_RIG_LOGIC.replace('/', "\\")),
        rig_logic,
    ));
    let chr = skeleton_entry(
        NPC_SKELETON_FILE.trim_end_matches(".chr"),
        &skeleton(NPC_SKELETON_SHIFT),
    );
    entries.push(chr);
    entries
}

/// Returns the Entries of [`npc_entries`] with the face of [`npc_face_logic`] and
/// [`npc_face_shapes`].
#[must_use]
pub fn npc_face_entries() -> Vec<TestEntry> {
    npc_face_entries_with(&npc_face_logic().build_rig(), &npc_face_shapes().build())
}
