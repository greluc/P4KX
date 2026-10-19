use std::cmp::Ordering;
use std::collections::HashMap;

use rayon::prelude::*;

use crate::query::schema::StructFields;
use crate::rows::NO_PARENT;
use crate::{DataForge, DataType, MAX_INHERITANCE_DEPTH};

/// The type and field names of one DataForge, read once and kept for completing Queries.
///
/// It owns its names, so it outlives the DataForge it was read from; a struct whose layout
/// cannot be read is listed without fields.
#[derive(Debug, Clone, Default)]
pub struct QuerySchema {
    pub(super) names: Box<[Box<str>]>,
    pub(super) structs: Box<[Entry]>,
    pub(super) types: Box<[u32]>,
}

/// One struct: its name, its parent and its fields, inherited ones included, sorted by name.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) name: Option<u32>,
    pub(super) parent: Option<usize>,
    pub(super) fields: Box<[Field]>,
}

/// One field of a struct, as a path walks it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Field {
    pub(super) name: u32,
    pub(super) data_type: DataType,
    pub(super) array: bool,
    pub(super) target: u16,
}

#[derive(Default)]
struct Interner<'f> {
    ids: HashMap<&'f str, u32>,
    names: Vec<Box<str>>,
}

impl<'f> Interner<'f> {
    fn id(&mut self, name: &'f str) -> u32 {
        if let Some(id) = self.ids.get(name) {
            return *id;
        }
        let id = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.names.push(name.into());
        self.ids.insert(name, id);
        id
    }
}

impl QuerySchema {
    /// Reads the struct names and the fields of every struct of `forge`.
    #[must_use]
    pub fn of(forge: &DataForge<'_>) -> Self {
        let structs = StructFields::read_all(forge);
        let mut interner = Interner::default();
        let mut entries: Vec<Entry> = (0..structs.len())
            .map(|index| {
                let name = structs.name(index).map(|name| interner.id(name));
                let parent = forge
                    .structs
                    .get(index)
                    .filter(|row| row.parent != NO_PARENT)
                    .and_then(|row| usize::try_from(row.parent).ok());
                let fields = structs
                    .fields(index)
                    .unwrap_or_default()
                    .iter()
                    .map(|field| Field {
                        name: interner.id(field.name),
                        data_type: field.slot.data_type(),
                        array: field.slot.is_array(),
                        target: field.slot.target(),
                    })
                    .collect();
                Entry {
                    name,
                    parent,
                    fields,
                }
            })
            .collect();
        let names = interner.names.into_boxed_slice();
        entries.par_iter_mut().for_each(|entry| {
            let mut fields = std::mem::take(&mut entry.fields).into_vec();
            fields.sort_by(|a, b| by_name(name_of(&names, a.name), name_of(&names, b.name)));
            fields.dedup_by_key(|field| field.name);
            entry.fields = fields.into_boxed_slice();
        });
        let mut types: Vec<u32> = entries.iter().filter_map(|entry| entry.name).collect();
        types.sort_by(|a, b| by_name(name_of(&names, *a), name_of(&names, *b)));
        types.dedup();
        Self {
            names,
            structs: entries.into_boxed_slice(),
            types: types.into_boxed_slice(),
        }
    }

    /// Returns how many structs the schema lists.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.structs.len()
    }

    /// Returns whether the schema lists no struct.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.structs.is_empty()
    }

    pub(super) fn name(&self, id: u32) -> &str {
        name_of(&self.names, id)
    }

    pub(super) fn entry(&self, index: usize) -> Option<&Entry> {
        self.structs.get(index)
    }

    /// Returns the first struct named `name`.
    pub(super) fn position_of(&self, name: &str) -> Option<usize> {
        self.structs
            .iter()
            .position(|entry| entry.name.is_some_and(|id| self.name(id) == name))
    }

    /// Returns the field `name` of the struct at `index`.
    pub(super) fn field(&self, index: usize, name: &str) -> Option<Field> {
        self.entry(index)?
            .fields
            .iter()
            .find(|field| self.name(field.name) == name)
            .copied()
    }

    /// Returns whether the struct at `index` is the struct at `base` or derives from it.
    pub(super) fn derives(&self, index: usize, base: usize) -> bool {
        let mut current = index;
        for _ in 0..MAX_INHERITANCE_DEPTH {
            if current == base {
                return true;
            }
            match self.entry(current).and_then(|entry| entry.parent) {
                Some(parent) => current = parent,
                None => return false,
            }
        }
        false
    }

    /// Returns the name of the struct at `index`, if it has one.
    pub(super) fn struct_name(&self, index: usize) -> Option<&str> {
        self.entry(index)?.name.map(|id| self.name(id))
    }
}

fn name_of(names: &[Box<str>], id: u32) -> &str {
    usize::try_from(id)
        .ok()
        .and_then(|id| names.get(id))
        .map_or("", |name| name)
}

/// Orders names ignoring ASCII case, and names equal so by their bytes.
pub(super) fn by_name(a: &str, b: &str) -> Ordering {
    a.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(b.bytes().map(|byte| byte.to_ascii_lowercase()))
        .then_with(|| a.cmp(b))
}
