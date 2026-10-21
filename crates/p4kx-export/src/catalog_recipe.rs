use crate::contract_recipe::{BlueprintTier, CostLine, PropertyModifier, ValueRange};
use crate::crafting_types::{self as library, Cost, CostContext, ModifierKind, Tier};
use crate::guid_text::CatalogGuid;

const SELECT: &str = "select";
const RESOURCE: &str = "resource";
const ITEM: &str = "item";
const MULTIPLIER: &str = "multiplier";
const ADDITIVE: &str = "additive";

/// Returns the tier `tier` as the catalog lists it.
pub(crate) fn tier_of<'f>(tier: &Tier<'f>) -> BlueprintTier<'f> {
    BlueprintTier {
        craft_time_seconds: tier.craft_time_seconds,
        mandatory: tier.mandatory.as_ref().map(line),
        optional: tier.optional.iter().map(line).collect(),
    }
}

fn line<'f>(cost: &Cost<'f>) -> CostLine<'f> {
    match cost {
        Cost::Select(selection) => CostLine {
            kind: SELECT,
            count: selection.count,
            slot: selection.slot,
            slot_key: selection.slot_key,
            options: selection.options.iter().map(line).collect(),
            ..with_context(&selection.context)
        },
        Cost::Resource(resource) => CostLine {
            kind: RESOURCE,
            resource_guid: resource.resource.map(CatalogGuid),
            quantity_scu: resource.quantity_scu,
            min_quality: resource.min_quality,
            ..with_context(&resource.context)
        },
        Cost::Item(item) => CostLine {
            kind: ITEM,
            item_guid: item.item.map(CatalogGuid),
            quantity_units: item.quantity,
            min_quality: item.min_quality,
            ..with_context(&item.context)
        },
    }
}

fn with_context<'f>(context: &CostContext<'f>) -> CostLine<'f> {
    CostLine {
        quantity_multiplier: context.quantity_multiplier,
        composition_inclusion: context.composition_inclusion,
        modifiers: context.modifiers.iter().map(modifier).collect(),
        ..CostLine::default()
    }
}

fn modifier(modifier: &library::PropertyModifier) -> PropertyModifier {
    PropertyModifier {
        property_guid: modifier.property.map(CatalogGuid),
        ranges: modifier
            .ranges
            .iter()
            .map(|range| ValueRange {
                kind: match range.kind {
                    ModifierKind::Multiplier => MULTIPLIER,
                    ModifierKind::Additive => ADDITIVE,
                },
                start_quality: range.start_quality,
                end_quality: range.end_quality,
                at_start: range.at_start,
                at_end: range.at_end,
            })
            .collect(),
    }
}
