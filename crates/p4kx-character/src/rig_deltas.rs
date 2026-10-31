// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::rig::SLOT_VARIANT_DELTAS;
use crate::rig_bytes::{f64_at, u16_at, u32_at, u64_at};
use crate::{DnaRig, RigError};

const SLOT: usize = SLOT_VARIANT_DELTAS;
const WHAT: &str = "head delta records";
const HEADER_LEN: usize = 0x28;
const RECORD_HEAD_LEN: usize = 4;
const INDEX_BITS: u32 = 9;
const INDEX_MASK: u32 = (1 << INDEX_BITS) - 1;
const STEPS: f64 = 4_194_303.0;

/// Returns the value of one step of a coefficient of a delta set that spans `scale`.
pub(crate) fn step_of(scale: f64) -> f64 {
    scale / STEPS
}

/// Which of the two delta sets of a head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaSet {
    /// The main deltas, which carry the appearance of the head.
    Main,
    /// The secondary deltas, which carry the corrections between poses.
    Secondary,
}

/// The header of the delta records of one head.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct VariantHeader {
    /// The number of main records.
    pub main_records: u32,
    /// The number of secondary records.
    pub secondary_records: u32,
    /// The range the coefficients of the main records span.
    pub main_scale: f64,
    /// The range the coefficients of the secondary records span.
    pub secondary_scale: f64,
    /// The offset of the main records in the file.
    pub main_offset: u64,
    /// The offset of the secondary records in the file.
    pub secondary_offset: u64,
}

impl VariantHeader {
    /// Returns the number of records of `set`.
    #[must_use]
    pub const fn records(&self, set: DeltaSet) -> u32 {
        match set {
            DeltaSet::Main => self.main_records,
            DeltaSet::Secondary => self.secondary_records,
        }
    }

    /// Returns the value of one step of a coefficient of `set`.
    #[must_use]
    pub fn step(&self, set: DeltaSet) -> f64 {
        step_of(match set {
            DeltaSet::Main => self.main_scale,
            DeltaSet::Secondary => self.secondary_scale,
        })
    }

    const fn offset(&self, set: DeltaSet) -> u64 {
        match set {
            DeltaSet::Main => self.main_offset,
            DeltaSet::Secondary => self.secondary_offset,
        }
    }
}

/// How each head moves the joints of the rig: per head, records of an output (a joint and an axis)
/// with the blendshapes that move it and their coefficients.
///
/// Read with [`DnaRig::variant_deltas`].
#[derive(Debug, Clone, Copy)]
pub struct VariantDeltas<'a> {
    file: &'a [u8],
    table: &'a [u8],
    variants: usize,
}

impl<'a> DnaRig<'a> {
    /// Reads the table of the delta records of each head.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`] or [`RigError::SlotTooShort`] when the table is missing or
    /// shorter than the heads need.
    pub fn variant_deltas(&self) -> Result<VariantDeltas<'a>, RigError> {
        let data = self.slot(SLOT, WHAT)?;
        let need = self.variants() * HEADER_LEN;
        let table = data.get(..need).ok_or(RigError::SlotTooShort {
            what: WHAT,
            slot: SLOT,
            need: need as u64,
            have: data.len(),
        })?;
        Ok(VariantDeltas {
            file: self.bytes(),
            table,
            variants: self.variants(),
        })
    }
}

impl<'a> VariantDeltas<'a> {
    /// Returns the header of `variant`, or `None` outside the table.
    #[must_use]
    pub fn header(&self, variant: usize) -> Option<VariantHeader> {
        if variant >= self.variants {
            return None;
        }
        let at = variant.checked_mul(HEADER_LEN)?;
        Some(VariantHeader {
            main_records: u32_at(self.table, at)?,
            secondary_records: u32_at(self.table, at.checked_add(4)?)?,
            main_scale: f64_at(self.table, at.checked_add(8)?)?,
            secondary_scale: f64_at(self.table, at.checked_add(16)?)?,
            main_offset: u64_at(self.table, at.checked_add(24)?)?,
            secondary_offset: u64_at(self.table, at.checked_add(32)?)?,
        })
    }

    /// Returns the records of `set` of `variant`, read lazily from the file.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::Malformed`] when `variant` is outside the table or the records start
    /// outside the file; a record that is cut short ends the iteration with the same error.
    pub fn records(&self, variant: usize, set: DeltaSet) -> Result<DeltaRecords<'a>, RigError> {
        let malformed = |reason| RigError::Malformed {
            what: WHAT,
            slot: SLOT,
            reason,
        };
        let header = self
            .header(variant)
            .ok_or_else(|| malformed("the head is outside the table"))?;
        let cursor = usize::try_from(header.offset(set))
            .ok()
            .filter(|&cursor| cursor <= self.file.len())
            .ok_or_else(|| malformed("the records start outside the file"))?;
        Ok(DeltaRecords {
            file: self.file,
            cursor,
            remaining: header.records(set),
        })
    }
}

/// An iterator over the records of one delta set of one head.
#[derive(Debug, Clone)]
pub struct DeltaRecords<'a> {
    file: &'a [u8],
    cursor: usize,
    remaining: u32,
}

impl<'a> DeltaRecords<'a> {
    /// Returns the iterator over `count` records that start at the beginning of `section`.
    pub(crate) const fn over(section: &'a [u8], count: u32) -> Self {
        Self {
            file: section,
            cursor: 0,
            remaining: count,
        }
    }
}

/// One record: an output and the blendshapes that move it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaRecord<'a> {
    /// The output the record moves: joint number times three plus the axis.
    pub output: u16,
    packed: &'a [u8],
}

/// One entry of a [`DeltaRecord`]: a blendshape and its quantised coefficient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaEntry {
    /// The blendshape.
    pub blendshape: u16,
    /// The coefficient in steps, see [`VariantHeader::step`].
    pub coefficient: i32,
}

impl DeltaRecord<'_> {
    /// Returns the entries of the record.
    pub fn entries(&self) -> impl Iterator<Item = DeltaEntry> {
        self.packed.as_chunks::<4>().0.iter().map(|&chunk| {
            let packed = u32::from_le_bytes(chunk);
            DeltaEntry {
                blendshape: u16::try_from(packed & INDEX_MASK).unwrap_or_default(),
                coefficient: packed.cast_signed() >> INDEX_BITS,
            }
        })
    }

    /// Returns the number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.packed.len() / 4
    }

    /// Returns whether the record has no entry.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.packed.is_empty()
    }
}

impl<'a> Iterator for DeltaRecords<'a> {
    type Item = Result<DeltaRecord<'a>, RigError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let record = read_record(self.file, self.cursor);
        match &record {
            Some((_, end)) => self.cursor = *end,
            None => self.remaining = 0,
        }
        Some(record.map(|(record, _)| record).ok_or(RigError::Malformed {
            what: WHAT,
            slot: SLOT,
            reason: "a record is cut short by the end of the file",
        }))
    }
}

fn read_record(file: &[u8], at: usize) -> Option<(DeltaRecord<'_>, usize)> {
    let output = u16_at(file, at)?;
    let count = usize::from(u16_at(file, at.checked_add(2)?)?);
    let start = at.checked_add(RECORD_HEAD_LEN)?;
    let end = start.checked_add(count.checked_mul(4)?)?;
    let packed = file.get(start..end)?;
    Some((DeltaRecord { output, packed }, end))
}
