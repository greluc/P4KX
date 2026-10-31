// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bytes::{put_u16, put_u32, put_u64};

/// The chunk type of the `RigLogic` chunk of a head skin.
pub const RIG_LOGIC_CHUNK: u32 = 0x0A24_85B6;

const HEADER_LEN: usize = 0xa8;
const SCALES_AT: usize = 0x48;
const INDEX_BITS: u32 = 9;

/// One delta record of a synthetic rig: the output it moves (joint number times three plus the
/// axis) and the blendshapes that move it with their quantised coefficients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLogicRecord {
    /// The output the record moves.
    pub output: u16,
    /// The blendshape and the coefficient in steps of each entry.
    pub entries: Vec<(u16, i32)>,
}

/// The joint delta records of a synthetic rig, with the layout of the real ones: the main set
/// carries translations in metres, the secondary set Euler angles in radians.
#[derive(Debug, Clone, PartialEq)]
pub struct TestLogicDeltas {
    /// The range the coefficients of the main records span; a step is this over 2^22 - 1.
    pub main_scale: f64,
    /// The range the coefficients of the secondary records span.
    pub secondary_scale: f64,
    /// The main records.
    pub main: Vec<TestLogicRecord>,
    /// The secondary records.
    pub secondary: Vec<TestLogicRecord>,
}

impl TestLogicDeltas {
    /// A step of 1e-6 in both sets; blendshape 1 moves the x translation of joint 1 by 0.01 m,
    /// blendshape 2 turns joint 1 about y by 0.5 rad.
    #[must_use]
    pub fn small() -> Self {
        let record = |output, blendshape, coefficient| TestLogicRecord {
            output,
            entries: vec![(blendshape, coefficient)],
        };
        Self {
            main_scale: 4.194_303,
            secondary_scale: 4.194_303,
            main: vec![record(3, 1, 10_000)],
            secondary: vec![record(4, 2, 500_000)],
        }
    }

    fn records(records: &[TestLogicRecord]) -> Vec<u8> {
        let mut out = Vec::new();
        for record in records {
            put_u16(&mut out, record.output);
            put_u16(&mut out, u16::try_from(record.entries.len()).unwrap_or(0));
            for &(blendshape, coefficient) in &record.entries {
                let packed = u32::from(blendshape) | (coefficient << INDEX_BITS).cast_unsigned();
                put_u32(&mut out, packed);
            }
        }
        out
    }
}

/// A synthetic `RigLogic` chunk: the control names and the corrective compositions of a head rig,
/// with the layout of the real chunks.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRigLogic {
    /// The names of the controls, the appearance blendshapes.
    pub controls: Vec<String>,
    /// For each corrective blendshape the five controls it combines.
    pub correctives: Vec<[u8; 5]>,
    /// The CRC-32 of the names of the joints the rig moves, in the order of the DNA rig's joints.
    pub joints: Vec<u32>,
    /// The delta records of the joints, as a story character's own `.RigLogic` file has them.
    pub deltas: Option<TestLogicDeltas>,
}

impl TestRigLogic {
    /// Three controls `CTL_PSD`, the constant control 0, `CTL_a` and `CTL_b`, and one corrective
    /// that combines `CTL_PSD`, `CTL_a` and `CTL_b`; no delta records.
    #[must_use]
    pub fn small() -> Self {
        Self {
            controls: ["CTL_PSD", "CTL_a", "CTL_b"].map(str::to_owned).to_vec(),
            correctives: vec![[0, 1, 2, 0, 0]],
            joints: ["FACIAL_root", "FACIAL_jaw"]
                .map(|name| crc32fast::hash(name.as_bytes()))
                .to_vec(),
            deltas: None,
        }
    }

    /// [`Self::small`] with the delta records of [`TestLogicDeltas::small`].
    #[must_use]
    pub fn with_deltas() -> Self {
        Self {
            deltas: Some(TestLogicDeltas::small()),
            ..Self::small()
        }
    }

    /// Returns the number of blendshapes: the controls and the correctives.
    #[must_use]
    pub const fn blendshapes(&self) -> usize {
        self.controls.len() + self.correctives.len()
    }

    /// Returns the bytes of the chunk: the rig behind a `u64` with its length.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let rig = self.build_rig();
        let mut out = (rig.len() as u64).to_le_bytes().to_vec();
        out.extend_from_slice(&rig);
        out
    }

    /// Returns the bytes of the rig alone, without the length in front.
    #[must_use]
    pub fn build_rig(&self) -> Vec<u8> {
        let crcs: Vec<u8> = self
            .controls
            .iter()
            .flat_map(|name| crc32fast::hash(name.as_bytes()).to_le_bytes())
            .collect();
        let mut names = Vec::new();
        for name in &self.controls {
            names.extend_from_slice(name.as_bytes());
            names.push(0);
        }
        let rows: Vec<u8> = self.correctives.iter().flatten().copied().collect();
        let (main, secondary) = self.deltas.as_ref().map_or_else(
            || (Vec::new(), Vec::new()),
            |deltas| {
                (
                    TestLogicDeltas::records(&deltas.main),
                    TestLogicDeltas::records(&deltas.secondary),
                )
            },
        );
        let hashes_start = HEADER_LEN;
        let names_start = hashes_start + crcs.len();
        let rows_start = names_start + names.len();
        let main_start = rows_start + rows.len();
        let secondary_start = main_start + main.len();
        let joints_start = secondary_start + secondary.len();
        let joints: Vec<u8> = self
            .joints
            .iter()
            .flat_map(|hash| hash.to_le_bytes())
            .collect();
        let joints_at = if joints.is_empty() { 0 } else { joints_start };
        let secondary_at = if self.deltas.is_some() {
            secondary_start
        } else {
            0
        };
        let mut out = self.header([
            names_start,
            hashes_start,
            rows_start,
            main_start,
            secondary_at,
            joints_at,
            0,
            0,
        ]);
        out.extend_from_slice(&crcs);
        out.extend_from_slice(&names);
        out.extend_from_slice(&rows);
        out.extend_from_slice(&main);
        out.extend_from_slice(&secondary);
        out.extend_from_slice(&joints);
        out.extend_from_slice(&[0; 8]);
        out
    }

    /// Returns the header with the section `starts`: the counts, the scales and counts of the
    /// delta records, and the table of starts.
    fn header(&self, starts: [usize; 8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIG V1.9");
        out.resize(0x3e, 0);
        put_u16(&mut out, 0xa8);
        put_u16(&mut out, u16::try_from(self.controls.len()).unwrap_or(0));
        put_u16(&mut out, u16::try_from(self.correctives.len()).unwrap_or(0));
        put_u16(&mut out, u16::try_from(self.joints.len()).unwrap_or(0));
        out.resize(SCALES_AT, 0);
        if let Some(deltas) = &self.deltas {
            out.extend_from_slice(&deltas.main_scale.to_le_bytes());
            out.extend_from_slice(&deltas.secondary_scale.to_le_bytes());
            put_u32(&mut out, u32::try_from(deltas.main.len()).unwrap_or(0));
            put_u32(&mut out, u32::try_from(deltas.secondary.len()).unwrap_or(0));
        }
        out.resize(0x68, 0);
        for start in starts {
            put_u64(&mut out, start as u64);
        }
        out
    }
}
