// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

use crate::rig_bytes::{u16_at, u64_at};

/// The first 8 bytes of a `RigLogic` chunk of a head skin.
pub const RIG_LOGIC_MAGIC: [u8; 8] = *b"RIG V1.9";

const HEADER_LEN: usize = 0xa8;
const HEADER_SIZE_AT: usize = 0x3e;
const APPEARANCE_AT: usize = 0x40;
const CORRECTIVE_AT: usize = 0x42;
const STARTS_AT: usize = 0x68;
const STARTS: usize = 8;
const NAMES_SLOT: usize = 0;
const CORRECTIVES_SLOT: usize = 2;
const ROW_LEN: usize = 5;

/// A failure to read the `RigLogic` chunk of a head skin.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RigLogicError {
    /// The chunk is shorter than its header.
    #[error("the RigLogic chunk is truncated: {have} bytes, the header needs {need}")]
    Truncated {
        /// The bytes the header needs.
        need: usize,
        /// The bytes the chunk has.
        have: usize,
    },
    /// The chunk does not start with the `RigLogic` magic.
    #[error("not a RigLogic chunk: the magic is not `RIG V1.9`")]
    BadMagic,
    /// The chunk breaks its layout.
    #[error("the RigLogic chunk is malformed: {0}")]
    Malformed(&'static str),
}

/// The names of the blendshapes of a head rig, read from the `RigLogic` chunk of its skin.
///
/// The rig has named controls, then correctives that fire when up to five of the controls are
/// active together; the blendshape numbers of the DNA rig's expression morphs run over both in
/// this order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigLogic {
    names: Vec<String>,
    correctives: Vec<[u8; ROW_LEN]>,
}

impl RigLogic {
    /// Reads the control names and the corrective compositions of a `RigLogic` chunk.
    ///
    /// # Errors
    ///
    /// Returns [`RigLogicError`] when the chunk is not a `RigLogic` chunk of the version this
    /// reader knows or its sections do not fit the chunk.
    pub fn read(chunk: &[u8]) -> Result<Self, RigLogicError> {
        if chunk.len() < HEADER_LEN {
            return Err(RigLogicError::Truncated {
                need: HEADER_LEN,
                have: chunk.len(),
            });
        }
        if chunk.get(..RIG_LOGIC_MAGIC.len()) != Some(RIG_LOGIC_MAGIC.as_slice()) {
            return Err(RigLogicError::BadMagic);
        }
        if u16_at(chunk, HEADER_SIZE_AT).map(usize::from) != Some(HEADER_LEN) {
            return Err(RigLogicError::Malformed("the header size is not 168"));
        }
        let controls = usize::from(u16_at(chunk, APPEARANCE_AT).unwrap_or_default());
        let correctives = usize::from(u16_at(chunk, CORRECTIVE_AT).unwrap_or_default());
        let names = names_of(section(chunk, NAMES_SLOT)?)?;
        if names.len() != controls {
            return Err(RigLogicError::Malformed(
                "the name pool does not hold one name per control",
            ));
        }
        let rows = section(chunk, CORRECTIVES_SLOT)?
            .get(..correctives * ROW_LEN)
            .ok_or(RigLogicError::Malformed(
                "the corrective table is shorter than its rows",
            ))?
            .as_chunks::<ROW_LEN>()
            .0
            .to_vec();
        if rows
            .iter()
            .flatten()
            .any(|&slot| usize::from(slot) >= controls)
        {
            return Err(RigLogicError::Malformed(
                "a corrective names a control the rig lacks",
            ));
        }
        Ok(Self {
            names,
            correctives: rows,
        })
    }

    /// Returns the number of named controls.
    #[must_use]
    pub const fn controls(&self) -> usize {
        self.names.len()
    }

    /// Returns the number of blendshapes: the controls and the correctives.
    #[must_use]
    pub const fn blendshapes(&self) -> usize {
        self.names.len() + self.correctives.len()
    }

    /// Returns the name of `blendshape`: the control's name, or for a corrective the names of the
    /// controls it combines, each once and joined by `*`; `None` outside the rig.
    #[must_use]
    pub fn name(&self, blendshape: usize) -> Option<String> {
        if let Some(name) = self.names.get(blendshape) {
            return Some(name.clone());
        }
        let row = self
            .correctives
            .get(blendshape.checked_sub(self.names.len())?)?;
        let mut parts: Vec<&str> = Vec::with_capacity(ROW_LEN);
        for &slot in row {
            if let Some(name) = self.names.get(usize::from(slot))
                && !parts.contains(&name.as_str())
            {
                parts.push(name);
            }
        }
        Some(parts.join("*"))
    }
}

/// Returns the bytes of section `slot`: from its start to the next start behind it.
fn section(chunk: &[u8], slot: usize) -> Result<&[u8], RigLogicError> {
    let start_of = |slot: usize| {
        u64_at(chunk, STARTS_AT + slot * 8)
            .and_then(|start| usize::try_from(start).ok())
            .unwrap_or(usize::MAX)
    };
    let start = start_of(slot);
    let end = (0..STARTS)
        .map(start_of)
        .filter(|&other| other > start && other <= chunk.len())
        .min()
        .unwrap_or(chunk.len());
    chunk
        .get(start..end)
        .ok_or(RigLogicError::Malformed("a section lies outside the chunk"))
}

fn names_of(pool: &[u8]) -> Result<Vec<String>, RigLogicError> {
    pool.split(|&byte| byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| {
            std::str::from_utf8(name)
                .map(str::to_owned)
                .map_err(|_| RigLogicError::Malformed("a control name is not UTF-8"))
        })
        .collect()
}
