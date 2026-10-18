use p4kx_extraction::{RecordTree, Value};

use crate::contract::{LocalisedText, Manufacturer};
use crate::guid_text::CatalogGuid;

/// Returns the manufacturer the Record tree `tree` describes.
pub(crate) fn manufacturer<'f>(tree: &RecordTree<'f>) -> Manufacturer<'f> {
    Manufacturer {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        code: string_field(tree, "Code"),
        name: LocalisedText::default(),
        description: LocalisedText::default(),
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

fn string_field<'f>(tree: &RecordTree<'f>, name: &str) -> Option<&'f str> {
    tree.fields()
        .iter()
        .find(|field| field.name() == name)
        .and_then(|field| match field.value() {
            Value::String(text) => Some(*text),
            _ => None,
        })
}
