use p4kx_extraction::{RecordTree, StructValue, Value};

use crate::guid_text::CatalogGuid;

/// Returns the Record name without the leading `<type>.`, e.g. `ACAS` for
/// `SCItemManufacturer.ACAS`; a name without that start is returned whole.
pub(crate) fn class_name<'f>(tree: &RecordTree<'f>) -> &'f str {
    let name = tree.name();
    name.strip_prefix(tree.type_name())
        .and_then(|rest| rest.strip_prefix('.'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(name)
}

/// Returns the struct the field `name` of `parent` holds.
pub(crate) fn structure<'t, 'f>(
    parent: Option<&'t StructValue<'f>>,
    name: &str,
) -> Option<&'t StructValue<'f>> {
    match parent?.field(name)? {
        Value::Struct(value) => Some(value),
        _ => None,
    }
}

/// Returns the first element of the `Components` array of `tree` whose struct is `type_name`.
pub(crate) fn component<'t, 'f>(
    tree: &'t RecordTree<'f>,
    type_name: &str,
) -> Option<&'t StructValue<'f>> {
    let Some(Value::Array(components)) = tree.field("Components") else {
        return None;
    };
    components.iter().find_map(|component| match component {
        Value::Struct(value) if value.type_name() == type_name => Some(value),
        _ => None,
    })
}

/// Returns the localisation key the field `name` of `parent` holds; an empty key is none.
pub(crate) fn locale_key<'f>(parent: Option<&StructValue<'f>>, name: &str) -> Option<&'f str> {
    match parent?.field(name)? {
        Value::Locale(key) if !key.is_empty() => Some(*key),
        _ => None,
    }
}

/// Returns the string or enum option the field `name` of `parent` holds; an empty one is none.
pub(crate) fn text<'f>(parent: Option<&StructValue<'f>>, name: &str) -> Option<&'f str> {
    match parent?.field(name)? {
        Value::String(text) | Value::Enum(text) if !text.is_empty() => Some(*text),
        _ => None,
    }
}

/// Returns the integer the field `name` of `parent` holds, `None` for one beyond `i64`.
pub(crate) fn integer(parent: Option<&StructValue<'_>>, name: &str) -> Option<i64> {
    match *parent?.field(name)? {
        Value::Int8(value) => Some(value.into()),
        Value::Int16(value) => Some(value.into()),
        Value::Int32(value) => Some(value.into()),
        Value::Int64(value) => Some(value),
        Value::UInt8(value) => Some(value.into()),
        Value::UInt16(value) => Some(value.into()),
        Value::UInt32(value) => Some(value.into()),
        Value::UInt64(value) => i64::try_from(value).ok(),
        _ => None,
    }
}

/// Returns the finite 32-bit float the field `name` of `parent` holds.
pub(crate) fn single(parent: Option<&StructValue<'_>>, name: &str) -> Option<f32> {
    match *parent?.field(name)? {
        Value::Single(value) if value.is_finite() => Some(value),
        _ => None,
    }
}

/// Returns the GUID of the Record the Reference in the field `name` of `parent` links to.
pub(crate) fn reference(parent: Option<&StructValue<'_>>, name: &str) -> Option<CatalogGuid> {
    match parent?.field(name)? {
        Value::Reference(reference) if reference.location().is_none() => {
            Some(CatalogGuid(reference.guid()))
        }
        _ => None,
    }
}
