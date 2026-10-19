use p4kx_extraction::{RecordTree, StructValue, Value};

use crate::contract::Commodity;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, component, element, elements, locale_key, reference};
use crate::selection::is_placeholder_key;
use crate::texts::Texts;

/// Returns the GUID of the `ResourceType` Record the default composition of the commodity
/// `tree` holds first.
pub(crate) fn resource_type_guid(tree: &RecordTree<'_>) -> Option<CatalogGuid> {
    let container = component(tree, "ResourceContainer");
    elements(container, "defaultComposition")
        .iter()
        .find_map(|entry| match entry {
            Value::Struct(entry) => reference(Some(entry), "entry"),
            _ => None,
        })
}

/// Returns the commodity the Record tree `tree` describes, its texts resolved by `texts`.
///
/// A name or description the commodity lacks or marks as a placeholder is taken from the
/// `ResourceType` Record `resource_type` instead.
pub(crate) fn commodity<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    resource_type: Option<&RecordTree<'f>>,
) -> Commodity<'f> {
    let purchasable = component(tree, "SCItemPurchasableParams");
    let display = element(
        Some(tree.root()),
        "StaticEntityClassData",
        "EntityUIDisplayParams",
    );
    let resource = resource_type.map(RecordTree::root);
    let name = usable(
        locale_key(purchasable, "displayName"),
        locale_key(resource, "displayName"),
    );
    let description = usable(
        locale_key(display, "displayDescription"),
        locale_key(resource, "description"),
    );
    Commodity {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(name),
        description: texts.localised(description),
        type_guid: resource_type_guid(tree),
        subtype_guid: first_tag(tree.root()),
        display_type: texts.localised(locale_key(purchasable, "displayType")),
    }
}

fn usable<'f>(own: Option<&'f str>, fallback: Option<&'f str>) -> Option<&'f str> {
    match own {
        Some(key) if !is_placeholder_key(key) => own,
        _ => fallback.or(own),
    }
}

fn first_tag(root: &StructValue<'_>) -> Option<CatalogGuid> {
    root.field("tags")
        .into_iter()
        .flat_map(|tags| match tags {
            Value::Array(items) => items.as_ref(),
            _ => &[],
        })
        .find_map(|tag| match tag {
            Value::Reference(tag) if tag.location().is_none() => Some(CatalogGuid(tag.guid())),
            _ => None,
        })
}
