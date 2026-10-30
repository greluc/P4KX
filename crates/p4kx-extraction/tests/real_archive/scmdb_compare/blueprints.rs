// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `CraftingBlueprintRecord`: the product, the tiers with their craft time, the slots of the
//! mandatory cost with their options and the modifier ranges of each slot.

use std::collections::{BTreeSet, HashMap};

use serde_json::Value;

use super::{Findings, elements, reference_guid};

const BLUEPRINTS: &str = "blueprints";
const PRODUCTS: &str = "blueprint products";
const SLOTS: &str = "blueprint slots";
const MODIFIERS: &str = "blueprint modifiers";

const SECONDS_PER_MINUTE: f64 = 60.0;
const SECONDS_PER_HOUR: f64 = 60.0 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: f64 = 24.0 * SECONDS_PER_HOUR;

/// The craft time fields of a `TimeValue_Partitioned` with their length in seconds.
const CRAFT_TIME_PARTS: [(&str, f64); 4] = [
    ("days", SECONDS_PER_DAY),
    ("hours", SECONDS_PER_HOUR),
    ("minutes", SECONDS_PER_MINUTE),
    ("seconds", 1.0),
];

/// The fields of a modifier range; an additive range spells the last two with `additive`.
const RANGE_FIELDS: [(&str, &str); 4] = [
    ("startQuality", "startQuality"),
    ("endQuality", "endQuality"),
    ("modifierAtStart", "additiveModifierAtStart"),
    ("modifierAtEnd", "additiveModifierAtEnd"),
];

pub(super) fn record_guids<'d>(dump: &'d Value, guids: &mut BTreeSet<&'d str>) {
    guids.extend(
        elements(&dump["blueprints"])
            .iter()
            .filter_map(|blueprint| blueprint["guid"].as_str()),
    );
}

pub(super) fn compare(dump: &Value, records: &HashMap<String, Value>, findings: &mut Findings) {
    for blueprint in elements(&dump["blueprints"]) {
        let guid = blueprint["guid"].as_str().unwrap_or_default();
        let ident = blueprint["tag"].as_str().unwrap_or(guid);
        let record = records.get(guid);
        findings.check(BLUEPRINTS, record.is_some(), || {
            format!("{ident}: {guid} is no Record")
        });
        let Some(record) = record else { continue };
        let body = &record["blueprint"];
        let expected = blueprint["productEntityClass"].as_str().unwrap_or_default();
        let product = &body["processSpecificData"]["entityClass"];
        findings.check(PRODUCTS, is_product(product, expected), || {
            format!("{ident}: product SCMDB={expected} p4kx={product}")
        });
        let tiers = elements(&body["tiers"]);
        let scmdb_tiers = elements(&blueprint["tiers"]);
        findings.check(BLUEPRINTS, tiers.len() == scmdb_tiers.len(), || {
            format!(
                "{ident}: tiers SCMDB={} p4kx={}",
                scmdb_tiers.len(),
                tiers.len()
            )
        });
        if tiers.len() != scmdb_tiers.len() {
            continue;
        }
        for (index, (scmdb, tier)) in scmdb_tiers.iter().zip(tiers).enumerate() {
            compare_tier(&format!("{ident} tier {index}"), scmdb, tier, findings);
        }
    }
}

/// Returns whether `product` links to `expected`; a dangling Reference in the game data is an
/// error value that names the GUID it refers to.
fn is_product(product: &Value, expected: &str) -> bool {
    product["error"].as_str().map_or_else(
        || reference_guid(product) == Some(expected),
        |error| error.contains(expected),
    )
}

fn compare_tier(ident: &str, scmdb: &Value, tier: &Value, findings: &mut Findings) {
    let costs = &tier["recipe"]["costs"];
    findings.number(
        BLUEPRINTS,
        ident,
        "craftTimeSeconds",
        scmdb["craftTimeSeconds"].as_f64(),
        craft_seconds(&costs["craftTime"]),
    );
    let slots = elements(&costs["mandatoryCost"]["options"]);
    let scmdb_slots = elements(&scmdb["slots"]);
    findings.check(SLOTS, slots.len() == scmdb_slots.len(), || {
        format!(
            "{ident}: slots SCMDB={} p4kx={}",
            scmdb_slots.len(),
            slots.len()
        )
    });
    if slots.len() != scmdb_slots.len() {
        return;
    }
    for (index, (scmdb, slot)) in scmdb_slots.iter().zip(slots).enumerate() {
        let ident = format!("{ident} slot {index}");
        compare_options(&ident, scmdb, slot, findings);
        compare_modifiers(&ident, scmdb, slot, findings);
    }
}

fn craft_seconds(time: &Value) -> Option<f64> {
    time.is_object().then(|| {
        CRAFT_TIME_PARTS
            .iter()
            .map(|(field, seconds)| time[*field].as_f64().unwrap_or_default() * seconds)
            .sum()
    })
}

fn compare_options(ident: &str, scmdb: &Value, slot: &Value, findings: &mut Findings) {
    let options = elements(&slot["options"]);
    let scmdb_options = elements(&scmdb["options"]);
    findings.check(SLOTS, options.len() == scmdb_options.len(), || {
        format!(
            "{ident}: options SCMDB={} p4kx={}",
            scmdb_options.len(),
            options.len()
        )
    });
    if options.len() != scmdb_options.len() {
        return;
    }
    for (scmdb, option) in scmdb_options.iter().zip(options) {
        findings.number(
            SLOTS,
            ident,
            "quantity",
            scmdb["quantity"].as_f64(),
            quantity(&option["quantity"]),
        );
        findings.number(
            SLOTS,
            ident,
            "minQuality",
            scmdb["minQuality"].as_f64(),
            option["minQuality"].as_f64(),
        );
    }
}

/// Returns a cost quantity: SCU of a resource, a count of items, or a plain number.
fn quantity(quantity: &Value) -> Option<f64> {
    if quantity.is_object() {
        quantity["standardCargoUnits"]
            .as_f64()
            .or_else(|| quantity["count"].as_f64())
    } else {
        quantity.as_f64()
    }
}

fn compare_modifiers(ident: &str, scmdb: &Value, slot: &Value, findings: &mut Findings) {
    let ranges = value_ranges(slot);
    let scmdb_ranges = elements(&scmdb["modifiers"]);
    findings.check(MODIFIERS, ranges.len() == scmdb_ranges.len(), || {
        format!(
            "{ident}: modifier ranges SCMDB={} p4kx={}",
            scmdb_ranges.len(),
            ranges.len()
        )
    });
    if ranges.len() != scmdb_ranges.len() {
        return;
    }
    for (scmdb, range) in scmdb_ranges.iter().zip(ranges) {
        let additive = !range["additiveModifierAtStart"].is_null();
        for (field, additive_field) in RANGE_FIELDS {
            let p4kx_field = if additive { additive_field } else { field };
            findings.number(
                MODIFIERS,
                ident,
                field,
                scmdb[field].as_f64(),
                range[p4kx_field].as_f64(),
            );
        }
    }
}

/// Returns the value ranges of every gameplay property modifier in the context of `slot`.
fn value_ranges(slot: &Value) -> Vec<&Value> {
    elements(&slot["context"])
        .iter()
        .flat_map(|context| {
            elements(&context["gameplayPropertyModifiers"]["gameplayPropertyModifiers"])
        })
        .flat_map(|modifier| elements(&modifier["valueRanges"]))
        .filter(|range| !range.is_null())
        .collect()
}
