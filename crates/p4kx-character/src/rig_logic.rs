// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

use crate::rig_bytes::{u16_at, u64_at};

/// The 8 bytes a rig starts with; in the `RigLogic` chunk of a head skin they follow a `u64` with
/// the length of the rig.
pub const RIG_LOGIC_MAGIC: [u8; 8] = *b"RIG V1.9";

const PREFIX_LEN: usize = 8;

/// The most padding bytes a chunk may hold behind the rig its prefix declares.
const PADDING_MAX: usize = 7;

const HEADER_LEN: usize = 0xa8;
const HEADER_SIZE_AT: usize = 0x3e;
const APPEARANCE_AT: usize = 0x40;
const CORRECTIVE_AT: usize = 0x42;
const JOINTS_AT: usize = 0x44;
const STARTS_AT: usize = 0x68;
const STARTS: usize = 8;
const NAMES_SLOT: usize = 0;
const HASHES_SLOT: usize = 1;
const CORRECTIVES_SLOT: usize = 2;
const JOINTS_SLOT: usize = 5;
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
    /// The length in front of the rig is longer than the rest of the chunk or leaves more than
    /// padding unaccounted.
    #[error("the RigLogic chunk declares a rig of {declared} bytes, it holds {have}")]
    PrefixMismatch {
        /// The length the prefix declares.
        declared: u64,
        /// The bytes that follow the prefix.
        have: usize,
    },
    /// The chunk breaks its layout.
    #[error("the RigLogic chunk is malformed: {0}")]
    Malformed(&'static str),
}

/// The names of the blendshapes of a head rig, read from the `RigLogic` chunk of its skin.
///
/// The rig has named controls, then correctives that fire when up to five of the controls are
/// active together; the blendshape numbers of the DNA rig's expression morphs run over both in
/// this order. The chunk also lists the joints the rig moves, by the CRC-32 of their names, in the
/// order of the joints of the DNA rig.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigLogic {
    names: Vec<String>,
    hashes: Vec<(u32, usize)>,
    correctives: Vec<[u8; ROW_LEN]>,
    joints: Vec<u32>,
}

impl RigLogic {
    /// Reads the control names and the corrective compositions of a `RigLogic` chunk, or of a
    /// rig file, which lacks the chunk's length prefix.
    ///
    /// # Errors
    ///
    /// Returns [`RigLogicError`] when the chunk is not a `RigLogic` chunk of the version this
    /// reader knows or its sections do not fit the chunk.
    pub fn read(chunk: &[u8]) -> Result<Self, RigLogicError> {
        let chunk = unprefixed(chunk)?;
        if chunk.len() < HEADER_LEN {
            return Err(RigLogicError::Truncated {
                need: HEADER_LEN,
                have: chunk.len(),
            });
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
        let joints = joint_hashes(
            chunk,
            usize::from(u16_at(chunk, JOINTS_AT).unwrap_or_default()),
        )?;
        if !names.is_empty() && u64_at(chunk, STARTS_AT + HASHES_SLOT * 8) == Some(0) {
            return Err(RigLogicError::Malformed("the hash table is missing"));
        }
        let hashes = control_hashes(section(chunk, HASHES_SLOT)?, &names)?;
        Ok(Self {
            names,
            hashes,
            correctives: rows,
            joints,
        })
    }

    /// Returns the control whose name has the CRC-32 `hash`, the hash a face animation names its
    /// tracks by; the first of equal hashes.
    #[must_use]
    pub fn control_by_hash(&self, hash: u32) -> Option<usize> {
        let first = self.hashes.partition_point(|&(other, _)| other < hash);
        self.hashes
            .get(first)
            .filter(|&&(other, _)| other == hash)
            .map(|&(_, control)| control)
    }

    /// Returns the CRC-32 of the name of each joint the rig moves, in the order of the joints of
    /// the DNA rig; empty when the chunk lists none.
    #[must_use]
    pub fn joint_hashes(&self) -> &[u32] {
        &self.joints
    }

    /// Returns the activation of every blendshape for the values of the controls: the controls
    /// themselves, with the constant control 0 at 1, then each corrective as the product of the
    /// controls it combines.
    ///
    /// `controls` is padded with zeros or cut to the number of controls.
    #[must_use]
    pub fn activations(&self, controls: &[f32]) -> Vec<f32> {
        let mut out = vec![0.0_f32; self.blendshapes()];
        self.activations_into(controls, &mut out);
        out
    }

    /// Writes [`Self::activations`] into `out`, which must hold [`Self::blendshapes`] values;
    /// a shorter slice is filled as far as it goes.
    pub fn activations_into(&self, controls: &[f32], out: &mut [f32]) {
        let count = self.names.len();
        for (slot, value) in out.iter_mut().take(count).zip(controls) {
            *slot = *value;
        }
        for slot in out.iter_mut().take(count).skip(controls.len()) {
            *slot = 0.0;
        }
        if let Some(constant) = out.first_mut() {
            *constant = 1.0;
        }
        let (given, derived) = out.split_at_mut(count.min(out.len()));
        for (slot, row) in derived.iter_mut().zip(&self.correctives) {
            *slot = row
                .iter()
                .map(|&control| given.get(usize::from(control)).copied().unwrap_or(0.0))
                .product();
        }
    }

    /// Returns the numbers of the controls of each corrective: the five controls it combines.
    #[must_use]
    pub fn corrective_rows(&self) -> &[[u8; 5]] {
        &self.correctives
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
    /// controls it combines, each once and joined by `*`, without the constant control 0 unless
    /// it is the only one; `None` outside the rig.
    #[must_use]
    pub fn name(&self, blendshape: usize) -> Option<String> {
        if let Some(name) = self.names.get(blendshape) {
            return Some(name.clone());
        }
        let row = self
            .correctives
            .get(blendshape.checked_sub(self.names.len())?)?;
        let mut parts: Vec<&str> = Vec::with_capacity(ROW_LEN);
        for &slot in row.iter().filter(|&&slot| slot != 0) {
            if let Some(name) = self.names.get(usize::from(slot))
                && !parts.contains(&name.as_str())
            {
                parts.push(name);
            }
        }
        if parts.is_empty() {
            parts.extend(self.names.first().map(String::as_str));
        }
        Some(parts.join("*"))
    }
}

/// Returns the rig of `chunk`: the chunk itself when it starts with the magic, else the bytes
/// behind a `u64` that holds their length, less the padding of at most [`PADDING_MAX`] bytes the
/// chunk may end with.
fn unprefixed(chunk: &[u8]) -> Result<&[u8], RigLogicError> {
    let has_magic = |bytes: &[u8]| bytes.get(..RIG_LOGIC_MAGIC.len()) == Some(&RIG_LOGIC_MAGIC);
    if has_magic(chunk) {
        return Ok(chunk);
    }
    let rig = chunk
        .get(PREFIX_LEN..)
        .filter(|rig| has_magic(rig))
        .ok_or(RigLogicError::BadMagic)?;
    let declared = u64_at(chunk, 0).ok_or(RigLogicError::Truncated {
        need: PREFIX_LEN,
        have: chunk.len(),
    })?;
    let mismatch = || RigLogicError::PrefixMismatch {
        declared,
        have: rig.len(),
    };
    let declared_len = usize::try_from(declared).map_err(|_| mismatch())?;
    let padding = rig.len().checked_sub(declared_len).ok_or_else(mismatch)?;
    if padding > PADDING_MAX {
        return Err(mismatch());
    }
    rig.get(..declared_len).ok_or_else(mismatch)
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

/// Returns the `count` joint hashes of the chunk, none when the chunk states no joints.
fn joint_hashes(chunk: &[u8], count: usize) -> Result<Vec<u32>, RigLogicError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let listed = u64_at(chunk, STARTS_AT + JOINTS_SLOT * 8).is_some_and(|start| start != 0);
    if !listed {
        return Err(RigLogicError::Malformed("the joint hashes are missing"));
    }
    let bytes = section(chunk, JOINTS_SLOT)?
        .get(..count * 4)
        .ok_or(RigLogicError::Malformed(
            "the joint table is shorter than its joints",
        ))?;
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&hash| u32::from_le_bytes(hash))
        .collect())
}

/// Returns the hash table of the controls sorted for lookup: the CRC-32 the chunk stores for each
/// control, which must be that of the control's name.
fn control_hashes(table: &[u8], names: &[String]) -> Result<Vec<(u32, usize)>, RigLogicError> {
    let stored = table
        .get(..names.len() * 4)
        .ok_or(RigLogicError::Malformed(
            "the hash table is shorter than its controls",
        ))?
        .as_chunks::<4>()
        .0;
    let mut hashes = Vec::with_capacity(names.len());
    for (control, (&bytes, name)) in stored.iter().zip(names).enumerate() {
        let hash = u32::from_le_bytes(bytes);
        if hash != crc32fast::hash(name.as_bytes()) {
            return Err(RigLogicError::Malformed(
                "a stored control hash is not the CRC-32 of the control's name",
            ));
        }
        hashes.push((hash, control));
    }
    hashes.sort_unstable();
    Ok(hashes)
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
