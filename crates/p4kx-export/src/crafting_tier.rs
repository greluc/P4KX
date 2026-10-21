use p4kx_extraction::{StructValue, is_placeholder_key};

use crate::blueprint::{as_struct, craft_time, whole};
use crate::crafting_types::{
    Cost, CostContext, ItemCost, ModifierKind, PropertyModifier, ResourceCost, Selection, Tier,
    ValueRange,
};
use crate::record_fields::{
    elements, integer, locale_key, number, reference, single, structure, text,
};

const MAX_DEPTH: usize = 32;
const SELECT: &str = "CraftingCost_Select";
const RESOURCE: &str = "CraftingCost_Resource";
const ITEM: &str = "CraftingCost_Item";
const MULTIPLIER: &str = "CraftingCostContext_QuantityMultiplier";
const INCLUSION: &str = "CraftingCostContext_ResultCompositionInclusion";
const MODIFIERS: &str = "CraftingCostContext_ResultGameplayPropertyModifiers";
const LINEAR: &str = "CraftingGameplayPropertyModifierValueRange_Linear";
const LINEAR_ADDITIVE: &str = "CraftingGameplayPropertyModifierValueRange_LinearIntegerAdditive";

/// Returns the tiers of the `CraftingBlueprint` `blueprint` in stored order.
pub(crate) fn tiers<'f>(blueprint: Option<&StructValue<'f>>) -> Vec<Tier<'f>> {
    elements(blueprint, "tiers")
        .iter()
        .filter_map(as_struct)
        .map(tier)
        .collect()
}

fn tier<'f>(tier: &StructValue<'f>) -> Tier<'f> {
    let costs = structure(structure(Some(tier), "recipe"), "costs");
    Tier {
        craft_time_seconds: craft_time(structure(costs, "craftTime")),
        mandatory: structure(costs, "mandatoryCost").and_then(|cost| cost_of(cost, 0)),
        optional: elements(costs, "optionalCosts")
            .iter()
            .filter_map(as_struct)
            .filter_map(|cost| cost_of(cost, 0))
            .collect(),
    }
}

fn cost_of<'f>(cost: &StructValue<'f>, depth: usize) -> Option<Cost<'f>> {
    let context = context(cost);
    match cost.type_name() {
        SELECT if depth < MAX_DEPTH => {
            let info = structure(Some(cost), "nameInfo");
            Some(Cost::Select(Selection {
                count: integer(Some(cost), "count"),
                slot: text(info, "debugName"),
                slot_key: locale_key(info, "displayName").filter(|key| !is_placeholder_key(key)),
                context,
                options: elements(Some(cost), "options")
                    .iter()
                    .filter_map(as_struct)
                    .filter_map(|option| cost_of(option, depth.saturating_add(1)))
                    .collect(),
            }))
        }
        RESOURCE => Some(Cost::Resource(ResourceCost {
            resource: reference(Some(cost), "resource").map(|guid| guid.0),
            quantity_scu: single(structure(Some(cost), "quantity"), "standardCargoUnits"),
            min_quality: integer(Some(cost), "minQuality"),
            context,
        })),
        ITEM => Some(Cost::Item(ItemCost {
            item: reference(Some(cost), "entityClass").map(|guid| guid.0),
            quantity: integer(Some(cost), "quantity"),
            min_quality: integer(Some(cost), "minQuality"),
            context,
        })),
        _ => None,
    }
}

fn context<'f>(cost: &StructValue<'f>) -> CostContext<'f> {
    let mut context = CostContext::default();
    for part in elements(Some(cost), "context").iter().filter_map(as_struct) {
        match part.type_name() {
            MULTIPLIER => context.quantity_multiplier = number(Some(part), "multiplier"),
            INCLUSION => context.composition_inclusion = text(Some(part), "option"),
            MODIFIERS => context.modifiers.extend(modifiers(part)),
            _ => {}
        }
    }
    context
}

fn modifiers(part: &StructValue<'_>) -> Vec<PropertyModifier> {
    let list = structure(Some(part), "gameplayPropertyModifiers");
    elements(list, "gameplayPropertyModifiers")
        .iter()
        .filter_map(as_struct)
        .map(|modifier| PropertyModifier {
            property: reference(Some(modifier), "gameplayPropertyRecord").map(|guid| guid.0),
            ranges: elements(Some(modifier), "valueRanges")
                .iter()
                .filter_map(as_struct)
                .filter_map(range)
                .collect(),
        })
        .collect()
}

fn range(range: &StructValue<'_>) -> Option<ValueRange> {
    let (kind, start, end) = match range.type_name() {
        LINEAR => (ModifierKind::Multiplier, "modifierAtStart", "modifierAtEnd"),
        LINEAR_ADDITIVE => (
            ModifierKind::Additive,
            "additiveModifierAtStart",
            "additiveModifierAtEnd",
        ),
        _ => return None,
    };
    Some(ValueRange {
        kind,
        start_quality: whole(Some(range), "startQuality"),
        end_quality: whole(Some(range), "endQuality"),
        at_start: number(Some(range), start),
        at_end: number(Some(range), end),
    })
}
