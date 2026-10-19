use p4kx_extraction::RecordTree;

use crate::contract::Item;
use crate::guid_text::CatalogGuid;
use crate::item_class::ClassLine;
use crate::record_fields::{
    class_name, component, integer, locale_key, reference, single, structure, text,
};
use crate::texts::Texts;

/// Returns the item the Record tree `tree` describes and the `Class:` line of its English
/// description, its texts resolved by `texts`.
pub(crate) fn item<'f>(tree: &RecordTree<'f>, texts: &Texts<'f>) -> (Item<'f>, ClassLine) {
    let definition = structure(component(tree, "SAttachableComponentParams"), "AttachDef");
    let localization = structure(definition, "Localization");
    let description = texts.localised(locale_key(localization, "Description"));
    let line = ClassLine::read(description.en.as_deref());
    let physics = structure(
        component(tree, "SEntityPhysicsControllerParams"),
        "PhysType",
    );
    let item = Item {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(locale_key(localization, "Name")),
        description,
        r#type: text(definition, "Type"),
        sub_type: text(definition, "SubType"),
        size: integer(definition, "Size"),
        grade: integer(definition, "Grade"),
        mass: single(physics, "Mass"),
        manufacturer_guid: reference(definition, "Manufacturer"),
        tags: text(definition, "Tags")
            .map(|tags| tags.split_whitespace().collect())
            .unwrap_or_default(),
        display_type: texts.localised(locale_key(
            component(tree, "SCItemPurchasableParams"),
            "displayType",
        )),
        class: line.class(),
    };
    (item, line)
}
