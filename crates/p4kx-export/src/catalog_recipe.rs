// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

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
            count: selection.count,
            slot: selection.slot,
            slot_key: selection.slot_key,
            options: selection.options.iter().map(line).collect(),
            ..bare(SELECT, &selection.context)
        },
        Cost::Resource(resource) => CostLine {
            resource_guid: resource.resource.map(CatalogGuid),
            quantity_scu: resource.quantity_scu,
            min_quality: resource.min_quality,
            ..bare(RESOURCE, &resource.context)
        },
        Cost::Item(item) => CostLine {
            item_guid: item.item.map(CatalogGuid),
            quantity_units: item.quantity,
            min_quality: item.min_quality,
            ..bare(ITEM, &item.context)
        },
    }
}

fn bare<'f>(kind: &'static str, context: &CostContext<'f>) -> CostLine<'f> {
    CostLine {
        kind,
        count: None,
        slot: None,
        slot_key: None,
        options: Vec::new(),
        resource_guid: None,
        item_guid: None,
        quantity_scu: None,
        quantity_units: None,
        min_quality: None,
        quantity_multiplier: context.quantity_multiplier,
        composition_inclusion: context.composition_inclusion,
        modifiers: context.modifiers.iter().map(modifier).collect(),
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
