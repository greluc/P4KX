// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::DataForge;

impl<'a> DataForge<'a> {
    /// Returns the bytes of instance `instance_index` of the struct at `struct_index`, or `None`
    /// when the struct or the instance does not exist.
    pub(crate) fn instance(&self, struct_index: u32, instance_index: u32) -> Option<&'a [u8]> {
        let index = usize::try_from(struct_index).ok()?;
        let size = u64::from(self.structs.get(index)?.size);
        if instance_index >= *self.instance_counts.get(index)? {
            return None;
        }
        let begin = u64::from(instance_index)
            .checked_mul(size)?
            .checked_add(*self.instance_starts.get(index)?)?;
        let end = begin.checked_add(size)?;
        self.instances
            .get(usize::try_from(begin).ok()?..usize::try_from(end).ok()?)
    }

    /// Returns the index of the Record whose own instance is instance `instance_index` of the
    /// struct at `struct_index`, or `None` when it belongs to no Record.
    pub(crate) fn record_at(&self, struct_index: u32, instance_index: u32) -> Option<u32> {
        let index = self.root_index.get_or_init(|| {
            let mut roots: Vec<(u32, u32, u32)> = self
                .records
                .iter_indexed()
                .map(|(record, def)| (def.struct_index, u32::from(def.instance_index), record))
                .collect();
            roots.sort_unstable();
            roots.into_boxed_slice()
        });
        let key = (struct_index, instance_index);
        let at = index.partition_point(|(s, i, _)| (*s, *i) < key);
        index
            .get(at)
            .filter(|(s, i, _)| (*s, *i) == key)
            .map(|(_, _, record)| *record)
    }
}
