use std::collections::HashMap;

use p4kx_extraction::{Guid, RecordTree, Value, is_placeholder_key};

use crate::contract::Commodity;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, component, element, elements, locale_key, reference};
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

/// The `ResourceTypeGroup` that directly lists each `ResourceType`, by the GUID of the
/// `ResourceType`.
pub(crate) type Groups = HashMap<Guid, CatalogGuid>;

/// Returns the `ResourceType` GUIDs the `resources` of the `ResourceTypeGroup` tree `tree` lists.
pub(crate) fn listed_resources(tree: &RecordTree<'_>) -> Vec<Guid> {
    elements(Some(tree.root()), "resources")
        .iter()
        .filter_map(|entry| match entry {
            Value::Reference(entry) if entry.location().is_none() => Some(entry.guid()),
            _ => None,
        })
        .collect()
}

/// Returns the commodity the Record tree `tree` describes, its texts resolved by `texts`.
///
/// A name or description the commodity lacks or marks as a placeholder is taken from the
/// `ResourceType` Record `resource_type` instead. Its type is the group in `groups` that lists
/// that `ResourceType`, its subtype the GUID of the `ResourceType`.
pub(crate) fn commodity<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    resource_type: Option<&RecordTree<'f>>,
    groups: &Groups,
) -> Commodity<'f> {
    let subtype = resource_type_guid(tree);
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
        type_guid: subtype.and_then(|subtype| groups.get(&subtype.0).copied()),
        subtype_guid: subtype,
        display_type: texts.localised(locale_key(purchasable, "displayType")),
    }
}

fn usable<'f>(own: Option<&'f str>, fallback: Option<&'f str>) -> Option<&'f str> {
    match own {
        Some(key) if !is_placeholder_key(key) => own,
        _ => fallback.or(own),
    }
}

/// Returns the commodity the `ResourceType` Record tree `tree` stands for, which no commodity
/// Record carries: its GUID, Record and texts are those of the `ResourceType`, its type the group
/// in `groups` that lists it, its subtype its own GUID.
pub(crate) fn resource_commodity<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    groups: &Groups,
) -> Commodity<'f> {
    let guid = tree.guid();
    Commodity {
        guid: CatalogGuid(guid),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(locale_key(Some(tree.root()), "displayName")),
        description: texts.localised(locale_key(Some(tree.root()), "description")),
        type_guid: groups.get(&guid).copied(),
        subtype_guid: Some(CatalogGuid(guid)),
        display_type: texts.localised(None),
    }
}
