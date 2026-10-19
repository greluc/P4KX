use p4kx_extraction::{RecordTree, StructValue, Value, is_placeholder_key};

use crate::contract::{Blueprint, Ingredient};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{
    class_name, elements, integer, locale_key, reference, single, structure, text,
};
use crate::texts::Texts;

const MAX_DEPTH: usize = 32;
const MAX_PART: f32 = 1.0e9;

const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_DAY: i64 = 86_400;

/// The slot an ingredient fills: the name info of the nearest enclosing selection that has one.
#[derive(Clone, Copy, Default)]
struct Slot<'f> {
    name: Option<&'f str>,
    key: Option<&'f str>,
}

/// Returns the blueprint the Record tree `tree` describes, its texts resolved by `texts`.
///
/// The first tier gives the craft time and the ingredients; a selection of costs is flattened
/// into one ingredient per resource or item, depth first in stored order.
pub(crate) fn blueprint<'f>(tree: &RecordTree<'f>, texts: &Texts<'f>) -> Blueprint<'f> {
    let blueprint = structure(Some(tree.root()), "blueprint");
    let creation = structure(blueprint, "processSpecificData");
    let tier = elements(blueprint, "tiers").iter().find_map(as_struct);
    let costs = structure(structure(tier, "recipe"), "costs");
    let mut ingredients = Vec::new();
    let slot = Slot::default();
    if let Some(cost) = structure(costs, "mandatoryCost") {
        collect(cost, slot, 0, &mut ingredients);
    }
    for cost in elements(costs, "optionalCosts")
        .iter()
        .filter_map(as_struct)
    {
        collect(cost, slot, 0, &mut ingredients);
    }
    let name = class_name(tree);
    Blueprint {
        guid: CatalogGuid(tree.guid()),
        class_name: name,
        record_path: tree.record_path(),
        key: name,
        name: texts.localised(locale_key(blueprint, "blueprintName")),
        category_guid: reference(blueprint, "category"),
        produced_item_guid: reference(creation, "entityClass"),
        craft_time_seconds: craft_time(structure(costs, "craftTime")),
        ingredients,
    }
}

const fn as_struct<'t, 'f>(value: &'t Value<'f>) -> Option<&'t StructValue<'f>> {
    match value {
        Value::Struct(value) => Some(value),
        _ => None,
    }
}

fn craft_time(time: Option<&StructValue<'_>>) -> Option<i64> {
    let part = |name: &str, factor: i64| whole(time, name)?.checked_mul(factor);
    let total = part("days", SECONDS_PER_DAY)?
        .checked_add(part("hours", SECONDS_PER_HOUR)?)?
        .checked_add(part("minutes", SECONDS_PER_MINUTE)?)?
        .checked_add(part("seconds", 1)?)?;
    (total >= 0).then_some(total)
}

/// Returns the integer or the float, rounded to a whole number, the field `name` of `parent`
/// holds.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a float within MAX_PART rounds to an i64 without loss"
)]
fn whole(parent: Option<&StructValue<'_>>, name: &str) -> Option<i64> {
    match parent?.field(name)? {
        Value::Single(value) if value.abs() <= MAX_PART => Some(value.round() as i64),
        Value::Double(value) if value.abs() <= f64::from(MAX_PART) => Some(value.round() as i64),
        _ => integer(parent, name),
    }
}

fn collect<'f>(
    cost: &StructValue<'f>,
    slot: Slot<'f>,
    depth: usize,
    out: &mut Vec<Ingredient<'f>>,
) {
    match cost.type_name() {
        "CraftingCost_Select" if depth < MAX_DEPTH => {
            let info = structure(Some(cost), "nameInfo");
            let own = Slot {
                name: text(info, "debugName"),
                key: locale_key(info, "displayName").filter(|key| !is_placeholder_key(key)),
            };
            let slot = if own.name.is_some() || own.key.is_some() {
                own
            } else {
                slot
            };
            for option in elements(Some(cost), "options").iter().filter_map(as_struct) {
                collect(option, slot, depth.saturating_add(1), out);
            }
        }
        "CraftingCost_Resource" => out.push(Ingredient {
            resource_guid: reference(Some(cost), "resource"),
            item_guid: None,
            quantity_scu: single(structure(Some(cost), "quantity"), "standardCargoUnits"),
            quantity_units: None,
            min_quality: integer(Some(cost), "minQuality"),
            slot: slot.name,
            slot_key: slot.key,
        }),
        "CraftingCost_Item" => out.push(Ingredient {
            resource_guid: None,
            item_guid: reference(Some(cost), "entityClass"),
            quantity_scu: None,
            quantity_units: integer(Some(cost), "quantity"),
            min_quality: integer(Some(cost), "minQuality"),
            slot: slot.name,
            slot_key: slot.key,
        }),
        _ => {}
    }
}
