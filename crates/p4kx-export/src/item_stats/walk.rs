use p4kx_extraction::{StructValue, Value};

const DEPTH: usize = 16;

/// Returns the first struct, the root first and then depth first in stored order, that holds a
/// field called `name`.
pub(super) fn owner_of<'t, 'f>(
    root: &'t StructValue<'f>,
    name: &str,
) -> Option<&'t StructValue<'f>> {
    owner_below(root, name, DEPTH)
}

fn owner_below<'t, 'f>(
    root: &'t StructValue<'f>,
    name: &str,
    depth: usize,
) -> Option<&'t StructValue<'f>> {
    if root.field(name).is_some() {
        return Some(root);
    }
    let depth = depth.checked_sub(1)?;
    root.fields()
        .iter()
        .find_map(|field| owner_in(field.value(), name, depth))
}

fn owner_in<'t, 'f>(
    value: &'t Value<'f>,
    name: &str,
    depth: usize,
) -> Option<&'t StructValue<'f>> {
    match value {
        Value::Struct(inner) => owner_below(inner, name, depth),
        Value::Array(items) => items.iter().find_map(|item| owner_in(item, name, depth)),
        _ => None,
    }
}
