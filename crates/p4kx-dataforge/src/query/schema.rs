use rayon::prelude::*;

use crate::query::compile::is_scalar;
use crate::rows::{NO_PARENT, StructDef};
use crate::schema::CONVERSION_ATTRIBUTE;
use crate::{DataForge, DataForgeError, DataType, MAX_INHERITANCE_DEPTH};

/// One field of a struct as a Record tree lists it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldDef<'f> {
    pub(crate) name: &'f str,
    pub(crate) position: u32,
    pub(crate) data_type: Option<DataType>,
    pub(crate) code: u16,
    pub(crate) array: bool,
}

impl FieldDef<'_> {
    /// Returns whether the field holds one scalar value.
    pub(crate) fn is_scalar(&self) -> bool {
        !self.array && self.data_type.is_some_and(is_scalar)
    }
}

/// The names and fields of every struct, inherited fields first, read once for compiling.
pub(crate) struct StructFields<'f> {
    forge: &'f DataForge<'f>,
    names: Vec<Option<&'f str>>,
    fields: Vec<Result<Vec<FieldDef<'f>>, DataForgeError>>,
}

impl<'f> StructFields<'f> {
    pub(crate) fn read_all(forge: &'f DataForge<'f>) -> Self {
        let rows: Vec<StructDef> = forge.structs.iter().collect();
        let names = rows
            .iter()
            .map(|row| forge.blob.get(row.name).ok())
            .collect();
        let fields = (0..rows.len())
            .into_par_iter()
            .map(|index| fields_of(forge, index))
            .collect();
        Self {
            forge,
            names,
            fields,
        }
    }

    pub(crate) const fn forge(&self) -> &'f DataForge<'f> {
        self.forge
    }

    pub(crate) const fn len(&self) -> usize {
        self.names.len()
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &'f str> + '_ {
        self.names.iter().flatten().copied()
    }

    pub(crate) fn positions_of<'s>(&'s self, name: &'s str) -> impl Iterator<Item = usize> + 's {
        self.names
            .iter()
            .enumerate()
            .filter(move |(_, candidate)| **candidate == Some(name))
            .map(|(index, _)| index)
    }

    pub(crate) fn position_of(&self, name: &str) -> Option<usize> {
        self.positions_of(name).next()
    }

    pub(crate) fn fields(&self, index: usize) -> Result<&[FieldDef<'f>], &DataForgeError> {
        match self.fields.get(index) {
            Some(Ok(fields)) => Ok(fields),
            Some(Err(error)) => Err(error),
            None => Ok(&[]),
        }
    }

    pub(crate) fn field(&self, index: usize, name: &str) -> Option<FieldDef<'f>> {
        self.fields(index)
            .ok()?
            .iter()
            .find(|field| field.name == name)
            .copied()
    }

    pub(crate) fn field_names(&self, index: usize) -> impl Iterator<Item = &'f str> + '_ {
        self.fields(index)
            .unwrap_or_default()
            .iter()
            .map(|field| field.name)
    }

    pub(crate) fn all_field_names(&self) -> impl Iterator<Item = &'f str> + '_ {
        (0..self.len()).flat_map(|index| self.field_names(index))
    }

    /// Returns the first field named `name` that holds one scalar value, otherwise the first
    /// field of that name, in struct order.
    pub(crate) fn representative(&self, name: &str) -> Option<FieldDef<'f>> {
        let mut declared = (0..self.len()).filter_map(|index| self.field(index, name));
        let first = declared.next()?;
        if first.is_scalar() {
            return Some(first);
        }
        Some(declared.find(FieldDef::is_scalar).unwrap_or(first))
    }
}

/// Returns whether the struct at `index` or one of its ancestors is among `targets`.
pub(crate) fn ancestors_include(forge: &DataForge<'_>, index: usize, targets: &[usize]) -> bool {
    let mut current = index;
    for _ in 0..MAX_INHERITANCE_DEPTH {
        if targets.contains(&current) {
            return true;
        }
        match forge.structs.get(current) {
            Some(row) if row.parent != NO_PARENT => {
                let Ok(parent) = usize::try_from(row.parent) else {
                    return false;
                };
                current = parent;
            }
            _ => return false,
        }
    }
    false
}

/// Returns the fields of the struct at `index` in Record tree order, the root ancestor's first.
fn fields_of<'f>(
    forge: &'f DataForge<'f>,
    index: usize,
) -> Result<Vec<FieldDef<'f>>, DataForgeError> {
    let struct_index = u32::try_from(index).unwrap_or(u32::MAX);
    let mut chain: Vec<StructDef> = Vec::new();
    let mut current = index;
    loop {
        let row = forge
            .structs
            .get(current)
            .ok_or(DataForgeError::IndexOutOfRange {
                part: "struct",
                index,
                target: "struct",
                target_index: current as u64,
            })?;
        if chain.len() == MAX_INHERITANCE_DEPTH {
            return Err(DataForgeError::InheritanceTooDeep {
                struct_index,
                limit: MAX_INHERITANCE_DEPTH,
            });
        }
        chain.push(row);
        if row.parent == NO_PARENT {
            break;
        }
        current = usize::try_from(row.parent).unwrap_or(usize::MAX);
        if current == index {
            return Err(DataForgeError::StructCycle { struct_index });
        }
    }
    let mut fields = Vec::new();
    let mut position = 0_u32;
    for row in chain.iter().rev() {
        let first = usize::from(row.first_property);
        for property in first..first + usize::from(row.property_count) {
            let def = forge
                .properties
                .get(property)
                .ok_or(DataForgeError::IndexOutOfRange {
                    part: "struct",
                    index,
                    target: "property",
                    target_index: property as u64,
                })?;
            fields.push(FieldDef {
                name: forge.blob.get(def.name)?,
                position,
                data_type: DataType::from_code(def.data_type),
                code: def.data_type,
                array: def.conversion != CONVERSION_ATTRIBUTE,
            });
            position = position.checked_add(1).ok_or(DataForgeError::TooLarge {
                part: "struct table",
            })?;
        }
    }
    Ok(fields)
}
