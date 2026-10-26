// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::layout::{CACHE_FIELDS, Layout};
use crate::{DataForge, DataForgeError};

/// The struct layouts one count reads, shared by all its parts.
///
/// A layout is read once and then borrowed by every thread; layouts beyond [`CACHE_FIELDS`]
/// fields in total are read again at each use, so the memory stays bounded however many structs a
/// crafted DataForge declares.
#[derive(Debug)]
pub(crate) struct SharedLayouts<'f> {
    slots: Box<[OnceLock<Layout<'f>>]>,
    fields: AtomicUsize,
}

impl<'f> SharedLayouts<'f> {
    /// Creates the empty cache for the structs of `forge`.
    pub(crate) fn new(forge: &DataForge<'_>) -> Self {
        Self {
            slots: (0..forge.structs.len()).map(|_| OnceLock::new()).collect(),
            fields: AtomicUsize::new(0),
        }
    }

    /// Returns the layout of the struct at `struct_index`.
    ///
    /// # Errors
    ///
    /// Returns the [`DataForgeError`] of a struct that does not exist or cannot be laid out.
    pub(crate) fn get<'s>(
        &'s self,
        forge: &'f DataForge<'f>,
        struct_index: u32,
    ) -> Result<Cow<'s, Layout<'f>>, DataForgeError> {
        let slot = usize::try_from(struct_index)
            .ok()
            .and_then(|index| self.slots.get(index));
        if let Some(layout) = slot.and_then(OnceLock::get) {
            return Ok(Cow::Borrowed(layout));
        }
        let layout = Layout::of(forge, struct_index)?;
        let Some(slot) = slot else {
            return Ok(Cow::Owned(layout));
        };
        let size = layout.fields.len();
        if self
            .fields
            .fetch_add(size, Ordering::Relaxed)
            .saturating_add(size)
            > CACHE_FIELDS
        {
            self.fields.fetch_sub(size, Ordering::Relaxed);
            return Ok(Cow::Owned(layout));
        }
        Ok(Cow::Borrowed(slot.get_or_init(|| layout)))
    }
}
