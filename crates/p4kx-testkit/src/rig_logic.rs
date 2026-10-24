// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bytes::{put_u16, put_u64};

/// The chunk type of the `RigLogic` chunk of a head skin.
pub const RIG_LOGIC_CHUNK: u32 = 0x0A24_85B6;

const HEADER_LEN: usize = 0xa8;

/// A synthetic `RigLogic` chunk: the control names and the corrective compositions of a head rig,
/// with the layout of the real chunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRigLogic {
    /// The names of the controls, the appearance blendshapes.
    pub controls: Vec<String>,
    /// For each corrective blendshape the five controls it combines.
    pub correctives: Vec<[u8; 5]>,
}

impl TestRigLogic {
    /// Three controls `CTL_a`, `CTL_b` and `CTL_c`, and one corrective that combines `CTL_a` and
    /// `CTL_c`.
    #[must_use]
    pub fn small() -> Self {
        Self {
            controls: ["CTL_a", "CTL_b", "CTL_c"].map(str::to_owned).to_vec(),
            correctives: vec![[0, 2, 0, 2, 2]],
        }
    }

    /// Returns the number of blendshapes: the controls and the correctives.
    #[must_use]
    pub const fn blendshapes(&self) -> usize {
        self.controls.len() + self.correctives.len()
    }

    /// Returns the bytes of the chunk.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let crcs = vec![0_u8; self.controls.len() * 4];
        let mut names = Vec::new();
        for name in &self.controls {
            names.extend_from_slice(name.as_bytes());
            names.push(0);
        }
        let rows: Vec<u8> = self.correctives.iter().flatten().copied().collect();
        let a = HEADER_LEN;
        let b = a + crcs.len();
        let c = b + names.len();
        let d = c + rows.len();
        let mut out = Vec::new();
        out.extend_from_slice(b"RIG V1.9");
        out.resize(0x3e, 0);
        put_u16(&mut out, 0xa8);
        put_u16(&mut out, u16::try_from(self.controls.len()).unwrap_or(0));
        put_u16(&mut out, u16::try_from(self.correctives.len()).unwrap_or(0));
        out.resize(0x68, 0);
        for start in [b, a, c, d, 0, 0, 0, 0] {
            put_u64(&mut out, start as u64);
        }
        out.extend_from_slice(&crcs);
        out.extend_from_slice(&names);
        out.extend_from_slice(&rows);
        out.extend_from_slice(&[0; 8]);
        out
    }
}
