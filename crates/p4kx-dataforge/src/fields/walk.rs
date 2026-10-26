// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::build::{element_bytes, pointer, reference_guid, words};
use crate::fields::FieldKind;
use crate::fields::part::{Part, Walk};
use crate::fields::seen::Seen;
use crate::layout::{FieldSlot, Layout};
use crate::scalar::read_scalar;
use crate::{DataForge, DataType, MAX_NESTING_DEPTH, ValueKind};

impl<'f> Part<'_, 'f> {
    pub(super) fn structure(
        &mut self,
        forge: &'f DataForge<'f>,
        struct_index: u32,
        layout: &Layout<'f>,
        bytes: &[u8],
        node: u32,
        depth: usize,
    ) -> Walk {
        self.spend(layout.fields.len())?;
        let pool = self.pool_of(node, struct_index, layout.fields.len());
        for (position, slot) in layout.fields.iter().enumerate() {
            let field = self.field_node(pool, position, node, slot.name)?;
            let value = bytes.get(slot.start..slot.end).unwrap_or_default();
            self.field(forge, slot, value, field, depth)?;
        }
        Ok(())
    }

    fn field(
        &mut self,
        forge: &'f DataForge<'f>,
        slot: &FieldSlot<'f>,
        bytes: &[u8],
        node: u32,
        depth: usize,
    ) -> Walk {
        if slot.array {
            return self.array(forge, slot, bytes, node, depth);
        }
        if slot.data_type == DataType::Class {
            self.leaf(
                node,
                FieldKind::new(DataType::Class, false),
                Seen::Empty,
                false,
            );
            return self.inline(forge, u32::from(slot.target), bytes, node, depth + 1);
        }
        self.single(forge, slot, bytes, node, false, depth)
    }

    /// Walks the fields of an inline struct below `node`.
    fn inline(
        &mut self,
        forge: &'f DataForge<'f>,
        struct_index: u32,
        bytes: &[u8],
        node: u32,
        depth: usize,
    ) -> Walk {
        if depth > MAX_NESTING_DEPTH {
            self.skipped += 1;
            return Ok(());
        }
        let layouts = self.layouts;
        let Ok(layout) = layouts.get(forge, struct_index) else {
            self.skipped += 1;
            return Ok(());
        };
        self.structure(forge, struct_index, &layout, bytes, node, depth)
    }

    fn single(
        &mut self,
        forge: &'f DataForge<'f>,
        slot: &FieldSlot<'f>,
        bytes: &[u8],
        node: u32,
        array: bool,
        depth: usize,
    ) -> Walk {
        self.spend(1)?;
        let kind = FieldKind::new(slot.data_type, array);
        match slot.data_type {
            DataType::StrongPointer => match pointer(bytes) {
                None => self.leaf(node, kind, Seen::Empty, true),
                Some((struct_index, instance)) => {
                    self.leaf(node, kind, Seen::Empty, false);
                    return self.owned(forge, struct_index, instance, node, depth + 1);
                }
            },
            DataType::WeakPointer => match pointer(bytes) {
                None => self.leaf(node, kind, Seen::Empty, true),
                Some((struct_index, instance)) => {
                    let seen = if forge.instance(struct_index, instance).is_some() {
                        Seen::Empty
                    } else {
                        Seen::Unreadable
                    };
                    self.leaf(node, kind, seen, false);
                }
            },
            DataType::Reference => match reference_guid(bytes) {
                None => self.leaf(node, kind, Seen::Empty, true),
                Some(guid) => self.leaf(node, kind, Seen::reference(&guid), false),
            },
            data_type => {
                let seen = read_scalar(forge, data_type, slot.target, bytes)
                    .map_or(Seen::Unreadable, |value| Seen::of(data_type, &value));
                let empty = matches!(seen, Seen::Empty);
                self.leaf(node, kind, seen, empty);
            }
        }
        Ok(())
    }

    /// Walks the instance a strong pointer owns, below the node of its struct's name.
    fn owned(
        &mut self,
        forge: &'f DataForge<'f>,
        struct_index: u32,
        instance: u32,
        parent: u32,
        depth: usize,
    ) -> Walk {
        let Some(bytes) = forge.instance(struct_index, instance) else {
            self.skipped += 1;
            return Ok(());
        };
        if depth > MAX_NESTING_DEPTH || !self.visited.insert((struct_index, instance)) {
            self.skipped += 1;
            return Ok(());
        }
        let layouts = self.layouts;
        let Ok(layout) = layouts.get(forge, struct_index) else {
            self.skipped += 1;
            return Ok(());
        };
        let node = self.type_node(parent, struct_index, layout.type_name)?;
        self.leaf(
            node,
            FieldKind::new(DataType::Class, false),
            Seen::Empty,
            false,
        );
        self.structure(forge, struct_index, &layout, bytes, node, depth)
    }

    fn array(
        &mut self,
        forge: &'f DataForge<'f>,
        slot: &FieldSlot<'f>,
        bytes: &[u8],
        node: u32,
        depth: usize,
    ) -> Walk {
        let [count, first] = words(bytes);
        let kind = ValueKind::of(slot.data_type);
        let len = kind.map_or_else(
            || {
                forge
                    .instance_counts
                    .get(usize::from(slot.target))
                    .map_or(0, |count| u64::from(*count))
            },
            |kind| forge.value_count(kind) as u64,
        );
        if u64::from(first) + u64::from(count) > len {
            self.skipped += 1;
            return Ok(());
        }
        for index in 0..count {
            let element = first.saturating_add(index);
            match kind {
                Some(kind) => {
                    let value = element_bytes(forge, kind, element);
                    self.single(forge, slot, value, node, true, depth + 1)?;
                }
                None => self.class_element(forge, slot, element, node, depth + 1)?,
            }
        }
        Ok(())
    }

    fn class_element(
        &mut self,
        forge: &'f DataForge<'f>,
        slot: &FieldSlot<'f>,
        element: u32,
        node: u32,
        depth: usize,
    ) -> Walk {
        self.spend(1)?;
        let struct_index = u32::from(slot.target);
        let Some(bytes) = forge.instance(struct_index, element) else {
            self.skipped += 1;
            return Ok(());
        };
        if !self.visited.insert((struct_index, element)) {
            self.skipped += 1;
            return Ok(());
        }
        self.leaf(
            node,
            FieldKind::new(DataType::Class, true),
            Seen::Empty,
            false,
        );
        self.inline(forge, struct_index, bytes, node, depth)
    }
}
