use crate::dataforge::out_of_range;
use crate::rows::{NO_PARENT, RecordDef, StructDef};
use crate::slot::Slot;
use crate::{DataForge, DataForgeError, Field, Guid, RecordTree};

/// The most structs one inheritance chain may hold, the struct itself included.
pub const MAX_INHERITANCE_DEPTH: usize = 64;

/// The most fields one Record may have, inherited ones included.
pub const MAX_RECORD_FIELDS: usize = 1 << 16;

/// The text a Record node carries besides its fields.
#[derive(Clone, Copy)]
pub(crate) struct RecordText<'f> {
    pub(crate) type_name: &'f str,
    pub(crate) record_path: &'f str,
    pub(crate) name: &'f str,
}

/// Builds the canonical tree of the Record at `index` with the row `def`.
pub(crate) fn record_tree<'f>(
    forge: &'f DataForge<'f>,
    index: u32,
    def: RecordDef,
    text: RecordText<'f>,
) -> Result<RecordTree<'f>, DataForgeError> {
    let chain = Chain::of(forge, index, def.struct_index)?;
    let instance = instance(forge, index, def, chain.record_struct())?;
    let mut fields = Vec::with_capacity(chain.field_count()?);
    let mut offset = 0_u64;
    for &(struct_index, structure) in chain.root_first() {
        let first = usize::from(structure.first_property);
        for property in first..first + usize::from(structure.property_count) {
            let slot = Slot::of(forge, struct_index, property)?;
            let end = offset.saturating_add(slot.size(forge)?);
            let bytes = usize::try_from(offset)
                .ok()
                .zip(usize::try_from(end).ok())
                .and_then(|(start, end)| instance.get(start..end))
                .ok_or(DataForgeError::StructLayout {
                    struct_index,
                    declared: structure.size,
                    fields: end,
                })?;
            fields.push(Field {
                name: slot.name(forge)?,
                value: slot.read(forge, bytes),
            });
            offset = end;
        }
        if offset != u64::from(structure.size) {
            return Err(DataForgeError::StructLayout {
                struct_index,
                declared: structure.size,
                fields: offset,
            });
        }
    }
    Ok(RecordTree {
        type_name: text.type_name,
        guid: Guid::from_stored(def.guid),
        record_path: text.record_path,
        name: text.name,
        fields,
    })
}

/// A struct and its ancestors with their rows, the root ancestor first.
struct Chain {
    structs: [(u32, StructDef); MAX_INHERITANCE_DEPTH],
    len: usize,
    record: StructDef,
}

impl Chain {
    /// Follows the parents of the struct of the Record at `record`.
    fn of(forge: &DataForge<'_>, record: u32, struct_index: u32) -> Result<Self, DataForgeError> {
        let empty = StructDef {
            name: 0,
            parent: NO_PARENT,
            property_count: 0,
            first_property: 0,
            size: 0,
        };
        let mut chain = Self {
            structs: [(0, empty); MAX_INHERITANCE_DEPTH],
            len: 0,
            record: empty,
        };
        let mut current = struct_index;
        let mut referrer = ("Record", record);
        loop {
            if chain
                .root_first()
                .iter()
                .any(|(index, _)| *index == current)
            {
                return Err(DataForgeError::StructCycle { struct_index });
            }
            let def = usize::try_from(current)
                .ok()
                .and_then(|index| forge.structs.get(index))
                .ok_or_else(|| {
                    out_of_range(referrer.0, referrer.1 as usize, "struct", current.into())
                })?;
            let slot =
                chain
                    .structs
                    .get_mut(chain.len)
                    .ok_or(DataForgeError::InheritanceTooDeep {
                        struct_index,
                        limit: MAX_INHERITANCE_DEPTH,
                    })?;
            *slot = (current, def);
            if chain.len == 0 {
                chain.record = def;
            }
            chain.len += 1;
            if def.parent == NO_PARENT {
                if let Some(chain) = chain.structs.get_mut(..chain.len) {
                    chain.reverse();
                }
                return Ok(chain);
            }
            referrer = ("struct", current);
            current = def.parent;
        }
    }

    fn root_first(&self) -> &[(u32, StructDef)] {
        self.structs.get(..self.len).unwrap_or_default()
    }

    const fn record_struct(&self) -> StructDef {
        self.record
    }

    fn field_count(&self) -> Result<usize, DataForgeError> {
        let count = self
            .root_first()
            .iter()
            .map(|(_, def)| usize::from(def.property_count))
            .sum();
        if count > MAX_RECORD_FIELDS {
            return Err(DataForgeError::TooManyFields {
                struct_index: self.root_first().last().map_or(0, |(index, _)| *index),
                count,
                limit: MAX_RECORD_FIELDS,
            });
        }
        Ok(count)
    }
}

/// Returns the bytes of the Record's instance of its struct.
pub(crate) fn instance<'f>(
    forge: &'f DataForge<'f>,
    index: u32,
    def: RecordDef,
    structure: StructDef,
) -> Result<&'f [u8], DataForgeError> {
    let missing = || {
        out_of_range(
            "Record",
            index as usize,
            "instance",
            def.instance_index.into(),
        )
    };
    let struct_index = usize::try_from(def.struct_index).map_err(|_| missing())?;
    let count = forge
        .instance_counts
        .get(struct_index)
        .copied()
        .unwrap_or_default();
    let start = forge
        .instance_starts
        .get(struct_index)
        .copied()
        .unwrap_or_default();
    if u32::from(def.instance_index) >= count {
        return Err(missing());
    }
    let size = u64::from(structure.size);
    let begin = u64::from(def.instance_index)
        .checked_mul(size)
        .and_then(|offset| offset.checked_add(start))
        .ok_or_else(missing)?;
    let end = begin.checked_add(size).ok_or_else(missing)?;
    usize::try_from(begin)
        .ok()
        .zip(usize::try_from(end).ok())
        .and_then(|(begin, end)| forge.instances.get(begin..end))
        .ok_or_else(missing)
}
