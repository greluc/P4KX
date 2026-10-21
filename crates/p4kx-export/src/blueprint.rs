use p4kx_extraction::{StructValue, Value};

use crate::catalog_recipe::tier_of;
use crate::contract::{Blueprint, Ingredient};
use crate::contract_recipe::BlueprintDismantle;
use crate::crafting_types::{BlueprintTiers, Cost, Tier};
use crate::guid_text::CatalogGuid;
use crate::record_fields::integer;
use crate::texts::Texts;

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

/// What links a blueprint to the rest of the crafting data.
pub(crate) struct Links {
    pub(crate) default_available: bool,
    pub(crate) dismantle: Option<BlueprintDismantle>,
}

/// Returns the blueprint `source` as the catalog lists it, its texts resolved by `texts`.
///
/// The first tier gives the craft time and the ingredients; a selection of costs is flattened
/// into one ingredient per resource or item, depth first in stored order. `tiers` keeps every
/// tier with its selections.
pub(crate) fn blueprint<'f>(
    source: &BlueprintTiers<'f>,
    texts: &Texts<'f>,
    links: Links,
) -> Blueprint<'f> {
    let first = source.tiers.first();
    Blueprint {
        guid: CatalogGuid(source.guid),
        class_name: source.class_name,
        record_path: source.record_path,
        key: source.class_name,
        name: texts.localised(source.name_key),
        category_guid: source.category.map(CatalogGuid),
        produced_item_guid: source.produced_item.map(CatalogGuid),
        craft_time_seconds: first.and_then(|tier| tier.craft_time_seconds),
        ingredients: ingredients(first),
        tiers: source.tiers.iter().map(tier_of).collect(),
        default_available: links.default_available,
        dismantle: links.dismantle,
    }
}

/// Returns the ingredients of `tier`: the mandatory cost, then the optional ones, selections
/// flattened.
fn ingredients<'f>(tier: Option<&Tier<'f>>) -> Vec<Ingredient<'f>> {
    let mut out = Vec::new();
    let Some(tier) = tier else {
        return out;
    };
    for cost in tier.mandatory.iter().chain(&tier.optional) {
        collect(cost, Slot::default(), &mut out);
    }
    out
}

fn collect<'f>(cost: &Cost<'f>, slot: Slot<'f>, out: &mut Vec<Ingredient<'f>>) {
    match cost {
        Cost::Select(selection) => {
            let own = Slot {
                name: selection.slot,
                key: selection.slot_key,
            };
            let slot = if own.name.is_some() || own.key.is_some() {
                own
            } else {
                slot
            };
            for option in &selection.options {
                collect(option, slot, out);
            }
        }
        Cost::Resource(line) => out.push(Ingredient {
            resource_guid: line.resource.map(CatalogGuid),
            item_guid: None,
            quantity_scu: line.quantity_scu,
            quantity_units: None,
            min_quality: line.min_quality,
            slot: slot.name,
            slot_key: slot.key,
        }),
        Cost::Item(line) => out.push(Ingredient {
            resource_guid: None,
            item_guid: line.item.map(CatalogGuid),
            quantity_scu: None,
            quantity_units: line.quantity,
            min_quality: line.min_quality,
            slot: slot.name,
            slot_key: slot.key,
        }),
    }
}

pub(crate) const fn as_struct<'t, 'f>(value: &'t Value<'f>) -> Option<&'t StructValue<'f>> {
    match value {
        Value::Struct(value) => Some(value),
        _ => None,
    }
}

pub(crate) fn craft_time(time: Option<&StructValue<'_>>) -> Option<i64> {
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
pub(crate) fn whole(parent: Option<&StructValue<'_>>, name: &str) -> Option<i64> {
    match parent?.field(name)? {
        Value::Single(value) if value.abs() <= MAX_PART => Some(value.round() as i64),
        Value::Double(value) if value.abs() <= f64::from(MAX_PART) => Some(value.round() as i64),
        _ => integer(parent, name),
    }
}
