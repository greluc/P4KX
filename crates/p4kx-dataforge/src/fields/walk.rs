// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};

use crate::build::{element_bytes, pointer, reference_guid, words};
use crate::fields::number::Extremes;
use crate::fields::seen::Seen;
use crate::fields::trie::{ROOT, Trie};
use crate::fields::{FieldKind, FieldStatisticsError};
use crate::layout::{FieldSlot, Layout, Layouts};
use crate::scalar::read_scalar;
use crate::{DataForge, DataType, MAX_NESTING_DEPTH, MAX_RECORD_VALUES, ValueKind};

const NONE: u32 = u32::MAX;

/// Why the walk of one Record stops.
enum Stop {
    TooLarge,
    Paths(FieldStatisticsError),
}

impl From<FieldStatisticsError> for Stop {
    fn from(error: FieldStatisticsError) -> Self {
        Self::Paths(error)
    }
}

type Walk = Result<(), Stop>;

/// The statistics of the Records one thread has walked, with the scratch space of the walk.
#[derive(Debug)]
pub(crate) struct Part<'f> {
    pub(crate) trie: Trie<'f>,
    layouts: Layouts<'f>,
    slots: HashMap<(u32, u32), usize>,
    children: Vec<Box<[u32]>>,
    types: HashMap<(u32, u32), u32>,
    visited: HashSet<(u32, u32)>,
    values: usize,
    record: u32,
    pub(crate) examined: u64,
    pub(crate) unreadable_records: u64,
    pub(crate) unreadable_values: u64,
}

impl<'f> Part<'f> {
    pub(crate) fn new() -> Self {
        Self {
            trie: Trie::new(),
            layouts: Layouts::default(),
            slots: HashMap::new(),
            children: Vec::new(),
            types: HashMap::new(),
            visited: HashSet::new(),
            values: 0,
            record: NONE,
            examined: 0,
            unreadable_records: 0,
            unreadable_values: 0,
        }
    }

    /// Adds two parts, dropping the scratch space of both.
    pub(crate) fn merge(self, other: Self) -> Result<Self, FieldStatisticsError> {
        let mut merged = Self::new();
        merged.examined = self.examined + other.examined;
        merged.unreadable_records = self.unreadable_records + other.unreadable_records;
        merged.unreadable_values = self.unreadable_values + other.unreadable_values;
        merged.trie = self.trie.merge(other.trie)?;
        Ok(merged)
    }

    /// Walks the Record at `index` and counts its values.
    ///
    /// # Errors
    ///
    /// Returns [`FieldStatisticsError::TooManyPaths`] when the Record has more paths than may be
    /// kept.
    pub(crate) fn add(
        &mut self,
        forge: &'f DataForge<'f>,
        index: u32,
    ) -> Result<(), FieldStatisticsError> {
        self.examined += 1;
        let Some(def) = forge.record_def(index) else {
            self.unreadable_records += 1;
            return Ok(());
        };
        let instance = u32::from(def.instance_index);
        let (Some(bytes), Ok(layout)) = (
            forge.instance(def.struct_index, instance),
            self.layouts.get(forge, def.struct_index),
        ) else {
            self.unreadable_records += 1;
            return Ok(());
        };
        self.record = index;
        self.values = 0;
        self.visited.clear();
        self.visited.insert((def.struct_index, instance));
        match self.structure(forge, def.struct_index, &layout, bytes, ROOT, 0) {
            Ok(()) => Ok(()),
            Err(Stop::TooLarge) => {
                self.unreadable_records += 1;
                Ok(())
            }
            Err(Stop::Paths(error)) => Err(error),
        }
    }

    const fn spend(&mut self, values: usize) -> Walk {
        self.values = self.values.saturating_add(values);
        if self.values > MAX_RECORD_VALUES {
            return Err(Stop::TooLarge);
        }
        Ok(())
    }

    fn structure(
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

    fn pool_of(&mut self, node: u32, struct_index: u32, len: usize) -> usize {
        *self.slots.entry((node, struct_index)).or_insert_with(|| {
            self.children.push(vec![NONE; len].into_boxed_slice());
            self.children.len() - 1
        })
    }

    fn field_node(
        &mut self,
        pool: usize,
        position: usize,
        parent: u32,
        name: &'f str,
    ) -> Result<u32, Stop> {
        let known = self
            .children
            .get(pool)
            .and_then(|nodes| nodes.get(position))
            .copied()
            .unwrap_or(NONE);
        if known != NONE {
            return Ok(known);
        }
        let id = self.trie.child(parent, name)?;
        if let Some(slot) = self
            .children
            .get_mut(pool)
            .and_then(|nodes| nodes.get_mut(position))
        {
            *slot = id;
        }
        Ok(id)
    }

    fn type_node(&mut self, parent: u32, struct_index: u32, name: &'f str) -> Result<u32, Stop> {
        if let Some(&known) = self.types.get(&(parent, struct_index)) {
            return Ok(known);
        }
        let id = self.trie.child(parent, name)?;
        self.types.insert((parent, struct_index), id);
        Ok(id)
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
            self.unreadable_values += 1;
            return Ok(());
        }
        let Ok(layout) = self.layouts.get(forge, struct_index) else {
            self.unreadable_values += 1;
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
            DataType::Reference => {
                let empty = reference_guid(bytes).is_none();
                self.leaf(node, kind, Seen::Empty, empty);
            }
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
            self.unreadable_values += 1;
            return Ok(());
        };
        if depth > MAX_NESTING_DEPTH || !self.visited.insert((struct_index, instance)) {
            self.unreadable_values += 1;
            return Ok(());
        }
        let Ok(layout) = self.layouts.get(forge, struct_index) else {
            self.unreadable_values += 1;
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
            self.unreadable_values += 1;
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
            self.unreadable_values += 1;
            return Ok(());
        };
        if !self.visited.insert((struct_index, element)) {
            self.unreadable_values += 1;
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

    /// Counts one value at `node`; `seen` says what it adds beyond its count.
    fn leaf(&mut self, node: u32, kind: FieldKind, seen: Seen, empty: bool) {
        let record = self.record;
        let Some(entry) = self.trie.nodes.get_mut(node as usize) else {
            return;
        };
        entry.count(record, kind, empty);
        match seen {
            Seen::Empty => {}
            Seen::Unreadable => self.unreadable_values += 1,
            Seen::Value {
                number,
                fingerprint,
            } => {
                if let Some(number) = number {
                    match &mut entry.extremes {
                        Some(extremes) => extremes.add(number),
                        unset => *unset = Some(Box::new(Extremes::of(number))),
                    }
                }
                entry
                    .distinct
                    .get_or_insert_with(Box::default)
                    .add(fingerprint);
            }
        }
    }
}
