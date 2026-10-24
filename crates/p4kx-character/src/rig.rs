// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::RigError;
use crate::rig_bytes::{u16_at, u32_at, u64_at};
use crate::rig_matrix::MAX_RIG_CELLS;

/// The first 12 bytes every DNA rig file starts with: `DNA V1.6` and four zero bytes.
pub const RIG_MAGIC: [u8; 12] = *b"DNA V1.6\0\0\0\0";

/// The slots of the pointer table.
pub(crate) const SLOT_COUNT: usize = 32;
const TABLE_AT: usize = 0x40;
const PAYLOAD_AT: usize = TABLE_AT + SLOT_COUNT * 8;

pub(crate) const SLOT_VARIANT_NAMES: usize = 2;
pub(crate) const SLOT_REGION_LIST: usize = 12;
pub(crate) const SLOT_VARIANT_JOINTS: usize = 15;
pub(crate) const SLOT_NEUTRAL_MORPHS: usize = 18;
pub(crate) const SLOT_EXPRESSION_MORPHS: usize = 19;
pub(crate) const SLOT_JOINT_WEIGHTS: usize = 21;
pub(crate) const SLOT_REGION_MASKS: usize = 22;
pub(crate) const SLOT_VARIANT_DELTAS: usize = 31;

const NAME_LEN: usize = 40;

/// The fixed fields at the start of a DNA rig file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RigHeader {
    /// The `u64` that identifies the build of the rig; its low half is the `variant_hash` of the
    /// DNA of the Characters made for it.
    pub identity: u64,
    /// The CRC-32C of the rig's name, e.g. `protos_human_male_face_t1_pu`; the `gender_hash` of
    /// the DNA of the Characters made for it.
    pub rig_hash: u32,
    /// The CRC-32C of the name of the mesh the file belongs to, e.g. `head`.
    pub mesh_hash: u32,
    /// The number of face part regions the variants are blended by, 13 for a human head.
    pub regions: u8,
    /// The number of regions this file's mesh covers; fewer than [`Self::regions`] for the eyes
    /// and teeth, which cover two each. Zero when the file does not say, which means all.
    pub covered_regions: u8,
    /// The number of heads the rig has: the base head 0 and the selectable archetypes.
    pub variants: u16,
    /// The number of vertices the header states for this level of detail.
    pub vertices: u32,
    /// The number of indices the header states for this level of detail.
    pub indices: u32,
}

#[derive(Debug, Clone, Copy)]
struct Slot {
    start: usize,
    end: usize,
}

/// A DNA rig (`.dna` file) of a head, eyes or teeth: the data that shapes a head from the DNA of a
/// Character.
///
/// The file holds a pointer table of 32 slots; the slots this reader knows are the names of the
/// heads, the joint offsets of each head, the vertex morphs of each head and region, the
/// blendshapes of each head, the joint and region weights and the delta records of each head. The
/// reader borrows the file, checks every offset and count against it and decodes on demand.
///
/// A Character's DNA ([`Dna`](crate::Dna)) selects, per face part, up to four heads to blend;
/// [`DnaMatrix`](crate::DnaMatrix) turns it into the weights the morphs are mixed with.
#[derive(Debug, Clone)]
pub struct DnaRig<'a> {
    bytes: &'a [u8],
    header: RigHeader,
    slots: [Option<Slot>; SLOT_COUNT],
}

impl<'a> DnaRig<'a> {
    /// Reads the header and pointer table of `bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::HeaderTruncated`], [`RigError::BadMagic`], [`RigError::ZeroVariants`]
    /// or [`RigError::SlotOutOfRange`] when the file is not a DNA rig.
    pub fn read(bytes: &'a [u8]) -> Result<Self, RigError> {
        if bytes.len() < PAYLOAD_AT {
            return Err(RigError::HeaderTruncated {
                need: PAYLOAD_AT,
                have: bytes.len(),
            });
        }
        if bytes.get(..RIG_MAGIC.len()) != Some(RIG_MAGIC.as_slice()) {
            return Err(RigError::BadMagic);
        }
        let header = RigHeader {
            identity: u64_at(bytes, 0x10).unwrap_or_default(),
            rig_hash: u32_at(bytes, 0x18).unwrap_or_default(),
            mesh_hash: u32_at(bytes, 0x1c).unwrap_or_default(),
            regions: bytes.get(0x28).copied().unwrap_or_default(),
            covered_regions: bytes.get(0x29).copied().unwrap_or_default(),
            variants: u16_at(bytes, 0x2a).unwrap_or_default(),
            vertices: u32_at(bytes, 0x30).unwrap_or_default(),
            indices: u32_at(bytes, 0x34).unwrap_or_default(),
        };
        if header.variants == 0 {
            return Err(RigError::ZeroVariants);
        }
        let cells = usize::from(header.variants) * usize::from(header.regions);
        if cells > MAX_RIG_CELLS {
            return Err(RigError::TooLarge {
                variants: usize::from(header.variants),
                regions: usize::from(header.regions),
                max: MAX_RIG_CELLS,
            });
        }
        let slots = Self::slots(bytes)?;
        Ok(Self {
            bytes,
            header,
            slots,
        })
    }

    fn slots(bytes: &[u8]) -> Result<[Option<Slot>; SLOT_COUNT], RigError> {
        let mut offsets = [0_usize; SLOT_COUNT];
        for (slot, offset) in offsets.iter_mut().enumerate() {
            let stored = TABLE_AT
                .checked_add(slot * 8)
                .and_then(|at| u64_at(bytes, at))
                .unwrap_or_default();
            let out_of_range = || RigError::SlotOutOfRange {
                slot,
                offset: stored,
                file_len: bytes.len(),
            };
            if stored == 0 {
                continue;
            }
            let at = usize::try_from(stored).map_err(|_| out_of_range())?;
            if at < PAYLOAD_AT || at > bytes.len() {
                return Err(out_of_range());
            }
            *offset = at;
        }
        Ok(std::array::from_fn(|slot| {
            let start = offsets.get(slot).copied().filter(|&start| start != 0)?;
            let end = offsets
                .iter()
                .copied()
                .filter(|&other| other > start)
                .min()
                .unwrap_or(bytes.len());
            Some(Slot { start, end })
        }))
    }

    /// Returns the header.
    #[must_use]
    pub const fn header(&self) -> &RigHeader {
        &self.header
    }

    /// Returns the number of heads, the base head 0 included.
    #[must_use]
    pub const fn variants(&self) -> usize {
        self.header.variants as usize
    }

    /// Returns the number of face part regions the heads are blended by.
    #[must_use]
    pub const fn regions(&self) -> usize {
        self.header.regions as usize
    }

    /// Returns the bytes of the file.
    pub(crate) const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Returns the bytes of slot `slot`, up to the next slot that starts behind it.
    pub(crate) fn slot(&self, slot: usize, what: &'static str) -> Result<&'a [u8], RigError> {
        let found = self
            .slots
            .get(slot)
            .copied()
            .flatten()
            .ok_or(RigError::SlotUnset { what, slot })?;
        self.bytes
            .get(found.start..found.end)
            .ok_or(RigError::SlotUnset { what, slot })
    }

    /// Returns the regions of the rig that this file's mesh covers, in the order of the file's
    /// per-region tables: all of them for a head, two for the eyes or the teeth.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::Malformed`] when the file covers more regions than the rig has or names
    /// a region it does not have.
    pub fn covered_regions(&self) -> Result<Vec<usize>, RigError> {
        let malformed = |reason| RigError::Malformed {
            what: "region list",
            slot: SLOT_REGION_LIST,
            reason,
        };
        let total = self.regions();
        let count = match usize::from(self.header.covered_regions) {
            0 => total,
            count => count,
        };
        if count > total {
            return Err(malformed("the file covers more regions than the rig has"));
        }
        let list = self.slot(SLOT_REGION_LIST, "region list").ok();
        let regions = (0..count)
            .map(|local| {
                let stored = list.and_then(|list| u32_at(list, local * 4));
                stored.map_or(Some(local), |region| usize::try_from(region).ok())
            })
            .collect::<Option<Vec<usize>>>()
            .ok_or_else(|| malformed("a region is out of range"))?;
        if regions.iter().any(|&region| region >= total) {
            return Err(malformed("a region is out of range"));
        }
        Ok(regions)
    }

    /// Returns the names of the heads in order; entry 0 is the base head, and the entry at index
    /// `head_id` is the head a Character's DNA names with `head_id`.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`], [`RigError::SlotTooShort`] or [`RigError::Malformed`] when
    /// the names are missing, truncated or not ASCII.
    pub fn variant_names(&self) -> Result<Vec<String>, RigError> {
        let data = self.slot(SLOT_VARIANT_NAMES, "head names")?;
        let need = self.variants() * NAME_LEN;
        let table = data.get(..need).ok_or(RigError::SlotTooShort {
            what: "head names",
            slot: SLOT_VARIANT_NAMES,
            need: need as u64,
            have: data.len(),
        })?;
        table
            .as_chunks::<NAME_LEN>()
            .0
            .iter()
            .map(|entry| {
                let len = entry.iter().position(|&byte| byte == 0).unwrap_or(NAME_LEN);
                let name = entry.get(..len).unwrap_or_default();
                if name.is_ascii() {
                    Ok(String::from_utf8_lossy(name).into_owned())
                } else {
                    Err(RigError::Malformed {
                        what: "head names",
                        slot: SLOT_VARIANT_NAMES,
                        reason: "a name is not ASCII",
                    })
                }
            })
            .collect()
    }
}
