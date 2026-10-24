// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    TestControl, TestDna, TestEntry, TestModel, TestPositions, armour_entries, armour_male_skeleton,
};

/// The vertices of the synthetic head skin, a cube.
pub const HEAD_VERTICES: usize = 24;
/// The regions of the synthetic head rig.
pub const HEAD_REGIONS: usize = 13;
/// The `gender_hash` of the male head rig, the CRC-32C of its name.
pub const MALE_RIG_HASH: u32 = 0xdd6c_67f6;
/// The `gender_hash` of the female head rig, the CRC-32C of its name.
pub const FEMALE_RIG_HASH: u32 = 0x9ef4_eb54;
/// The folder of the male head rig below `Data`, with `/` as separator.
pub const MALE_HEAD_FOLDER: &str =
    "Objects/Characters/Human/heads/male/pu/protos_human_male_face_t1_pu";
/// The name of the male head rig.
pub const MALE_RIG: &str = "protos_human_male_face_t1_pu";

fn head_skin() -> TestModel {
    TestModel::cube()
        .with_positions(TestPositions::Float)
        .with_nodes(Vec::new())
        .with_skeleton(&armour_male_skeleton())
        .with_bone_map(false, vec![vec![(2, 255)]; HEAD_VERTICES])
        .with_index(0, 5)
}

/// Returns a synthetic male head rig of five heads and thirteen regions for a mesh of `vertices`
/// vertices, named by `rig_hash`.
///
/// Only the neutral morphs move something: the control of head 2 in region 3 moves the stream
/// vertex 5 by `code * scale` with the codes 100, 50 and 10 and the scale 0.001, 0.002 and 0.003
/// per axis; controls of other heads and regions are set on vertices 5 and 6, with larger scales.
#[must_use]
pub fn head_rig(vertices: usize, rig_hash: u32) -> TestDna {
    let control = |scale: [f32; 3]| TestControl {
        scale,
        bias: [0.0; 3],
    };
    let mut dna = TestDna::small();
    dna.rig_hash = rig_hash;
    dna.names = ["base", "a", "b", "c", "d"].map(str::to_owned).to_vec();
    dna.regions = 13;
    dna.covered = Vec::new();
    dna.vertices = vertices;
    dna.joints = vec![vec![[0.0; 3]]; 5];
    dna.joint_weights = vec![vec![0.0; HEAD_REGIONS]];
    dna.masks = vec![vec![1.0 / 13.0; vertices]; HEAD_REGIONS];
    dna.controls = (0..4 * HEAD_REGIONS)
        .map(|index| {
            if index == 16 {
                control([0.001, 0.002, 0.003])
            } else {
                control([1.0; 3])
            }
        })
        .collect();
    dna.codes = (0..vertices)
        .map(|vertex| match vertex {
            5 => vec![(0, [9, 9, 9]), (16, [100, 50, 10])],
            6 => vec![(0, [3, 3, 3])],
            _ => Vec::new(),
        })
        .collect();
    dna.blendshapes = 1;
    dna.cells.clear();
    dna.deltas = vec![[Vec::new(), Vec::new()]; 5];
    dna.delta_scales = vec![(1.0, 1.0); 5];
    dna
}

/// Returns the Entries of a synthetic Archive with the bodies of [`armour_entries`] and the head
/// skin of the male head rig.
///
/// The skin is a cube whose first triangle uses vertex 5; `rig` is the bytes of the head's `.dna`
/// file, left out when `None`. The teeth are the same cube, with the same rig.
#[must_use]
pub fn head_entries(rig: Option<Vec<u8>>) -> Vec<TestEntry> {
    let model = head_skin();
    let stem = format!(
        "Data\\{}\\{MALE_RIG}_head",
        MALE_HEAD_FOLDER.replace('/', "\\")
    );
    let teeth = stem.replace("_head", "_teeth");
    let mut entries = armour_entries();
    for part in [&stem, &teeth] {
        entries.push(TestEntry::stored(format!("{part}.skin"), model.cgf()));
        entries.push(TestEntry::stored(format!("{part}.skinm"), model.cgfm()));
    }
    if let Some(rig) = rig {
        entries.push(TestEntry::stored(format!("{teeth}.dna"), rig.clone()));
        entries.push(TestEntry::stored(format!("{stem}.dna"), rig));
    }
    entries
}
