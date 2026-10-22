use p4kx_extraction::{DataForge, RecordTree, TreeBuilder};

use crate::contract::StarmapObject;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{boolean, class_name, locale_key, reference, text};
use crate::starmap_containers::Placements;
use crate::texts::Texts;

/// The precision of a position, a millimetre.
const MILLI: f64 = 1000.0;

/// Returns the starmap object the Record tree `tree` describes: its name resolved by `texts`, its
/// type from the Record the Reference `type` links to in `forge`, its place from `placements`.
pub(crate) fn starmap_object<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    forge: &'f DataForge<'f>,
    placements: &Placements,
) -> StarmapObject<'f> {
    let root = Some(tree.root());
    let key =
        locale_key(root, "name").or_else(|| text(root, "name").filter(|v| v.starts_with('@')));
    let type_guid = reference(root, "type");
    let placement = placements.get(tree.guid());
    StarmapObject {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(key),
        r#type: type_guid.and_then(|guid| type_of(forge, guid)),
        type_guid,
        parent_guid: reference(root, "parent"),
        hide_in_starmap: boolean(root, "hideInStarmap"),
        system: placement.map(|placement| placement.system.to_string()),
        entity_name: placement
            .map(|placement| placement.entity_name.as_str())
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
        placed_under_guid: placement
            .and_then(|placement| placement.placed_under)
            .map(CatalogGuid),
        position: placement.map(|placement| placement.position.map(millimetres)),
    }
}

/// Returns the `name` of the type Record `guid` links to when it is a plain string, else the Record
/// name without its type.
fn type_of<'f>(forge: &'f DataForge<'f>, guid: CatalogGuid) -> Option<&'f str> {
    let record = forge.record_by_guid(guid.0)?;
    let tree = TreeBuilder::new().build(&record).ok()?;
    let plain = text(Some(tree.root()), "name").filter(|name| !name.starts_with('@'));
    Some(plain.unwrap_or_else(|| class_name(&tree)))
}
fn millimetres(metres: f64) -> f64 {
    (metres * MILLI).round() / MILLI + 0.0
}
