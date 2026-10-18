use p4kx_extraction::{Field, RecordTree, StructValue, Value};

use crate::contract::{LocalisedText, Manufacturer};
use crate::guid_text::CatalogGuid;

/// Returns the manufacturer the Record tree `tree` describes.
pub(crate) fn manufacturer<'f>(tree: &RecordTree<'f>) -> Manufacturer<'f> {
    let localization = match field(tree, "Localization") {
        Some(Value::Struct(localization)) => Some(localization),
        _ => None,
    };
    Manufacturer {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        code: match field(tree, "Code") {
            Some(Value::String(code)) => Some(*code),
            _ => None,
        },
        name: localised(localization, "Name"),
        description: localised(localization, "Description"),
    }
}

/// Returns the Record name without the leading `<type>.`, e.g. `ACAS` for
/// `SCItemManufacturer.ACAS`; a name without that start is returned whole.
pub(crate) fn class_name<'f>(tree: &RecordTree<'f>) -> &'f str {
    let name = tree.name();
    name.strip_prefix(tree.type_name())
        .and_then(|rest| rest.strip_prefix('.'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(name)
}

fn field<'t, 'f>(tree: &'t RecordTree<'f>, name: &str) -> Option<&'t Value<'f>> {
    tree.fields()
        .iter()
        .find(|field| field.name() == name)
        .map(Field::value)
}

/// Returns the text whose localisation key the field `name` of `localization` holds; an empty
/// key is none.
fn localised<'f>(localization: Option<&StructValue<'f>>, name: &str) -> LocalisedText<'f> {
    let key = localization
        .and_then(|localization| localization.field(name))
        .and_then(|value| match value {
            Value::Locale(key) if !key.is_empty() => Some(*key),
            _ => None,
        });
    LocalisedText {
        key,
        ..LocalisedText::default()
    }
}
