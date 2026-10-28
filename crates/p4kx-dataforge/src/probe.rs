// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::build::{element_bytes, pointer, words};
use crate::{DataForge, DataForgeError, DataType, Record, TreeBuilder, ValueKind};

/// A cheap test whether a Record's strong pointer array holds an instance of one struct, without
/// building the Record tree.
#[derive(Debug, Clone, Copy)]
pub struct PointerProbe<'f> {
    field: &'f str,
    struct_index: u32,
}

impl<'f> DataForge<'f> {
    /// Returns a probe for Records whose array field `field` of strong pointers holds an
    /// instance of the struct named `struct_name`, or `None` when no struct of that name has an
    /// instance: then no Record can match and the probe is not needed.
    #[must_use]
    pub fn pointer_probe(&'f self, field: &'f str, struct_name: &str) -> Option<PointerProbe<'f>> {
        self.structs()
            .find(|structure| {
                structure.instance_count() > 0
                    && structure.name().is_ok_and(|name| name == struct_name)
            })
            .map(|structure| PointerProbe {
                field,
                struct_index: structure.index(),
            })
    }
}

impl<'f> TreeBuilder<'f> {
    /// Returns whether the array field of `record` named by `probe` holds a strong pointer to an
    /// instance of the probe's struct. Only the pointer array is read, never the tree; the match
    /// is exact, so an instance of a derived struct does not count.
    ///
    /// # Errors
    ///
    /// Returns the [`DataForgeError`] of the Record's struct layout.
    pub fn holds(
        &mut self,
        probe: &PointerProbe<'f>,
        record: &Record<'f>,
    ) -> Result<bool, DataForgeError> {
        let forge = record.forge;
        let def = record.def;
        let layout = self.layouts.get(forge, def.struct_index)?;
        let Some(slot) = layout.fields.iter().find(|slot| {
            slot.name == probe.field && slot.array && slot.data_type == DataType::StrongPointer
        }) else {
            return Ok(false);
        };
        let Some(bytes) = forge
            .instance(def.struct_index, u32::from(def.instance_index))
            .and_then(|bytes| bytes.get(slot.start..slot.end))
        else {
            return Ok(false);
        };
        let [count, first] = words(bytes);
        Ok((first..first.saturating_add(count)).any(|element| {
            pointer(element_bytes(forge, ValueKind::StrongPointer, element))
                .is_some_and(|(struct_index, _)| struct_index == probe.struct_index)
        }))
    }
}
