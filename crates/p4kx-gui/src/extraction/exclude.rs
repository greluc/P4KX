// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Entry, Exclusion, FilterError};

use crate::EntryIndex;

/// Parses the text of the exclusion field: one Filter per line, blank lines ignored.
///
/// # Errors
///
/// Returns the [`FilterError`] of the first Filter that is not accepted.
pub(crate) fn parse(text: &str) -> Result<Exclusion, FilterError> {
    Exclusion::parse(text.lines().filter(|line| !line.is_empty()))
}

/// The Entries that remain after an exclusion.
pub(crate) struct Left {
    /// The ids of the Entries that remain, in their order.
    pub(crate) ids: Vec<u32>,
    /// The number of left-out parts of textures that remain; a Conversion still reads them.
    pub(crate) excluded_parts: usize,
}

/// Leaves the Entries `exclusion` matches out of `ids` of `index`; `None` when none remain.
pub(crate) fn leave_out(index: &EntryIndex, ids: Vec<u32>, exclusion: &Exclusion) -> Option<Left> {
    let left = exclusion.leave_out(ids, |&id| index.entry(id).map_or("", Entry::path));
    (!left.kept.is_empty()).then_some(Left {
        ids: left.kept,
        excluded_parts: left.parts_of_kept_textures.len(),
    })
}
