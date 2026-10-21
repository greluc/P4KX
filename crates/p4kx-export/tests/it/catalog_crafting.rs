//! Commodities and blueprints of the Basetool catalog, through the public API.

use crate::crafting_fixture;

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{BuiltCatalog, CatalogOptions, Filter, Kind, build_catalog};
use p4kx_extraction::Archive;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn build(filter: Option<&Filter>) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = Archive::open_with_key(
        crafting_fixture::write_archive(dir.path())?,
        Some(p4kx_testkit::test_key()),
    )?;
    let options = CatalogOptions {
        filter,
        ..CatalogOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000))
    };
    let built = build_catalog(&archive, &options)?;
    let json = serde_json::from_slice(built.json())?;
    Ok((built, json))
}

fn class_names<'a>(catalog: &'a Value, kind: &str) -> Vec<&'a str> {
    catalog[kind]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|entry| entry["className"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn commodities_are_selected_by_prefix_and_type_without_leftovers_ordered_by_guid() -> TestResult {
    let (built, catalog) = build(None)?;
    assert_eq!(
        class_names(&catalog, "commodities"),
        ["gold", "dopple", "ember", "iron"],
        "under entities/commodities, without the test commodity and the carryable, then the used ResourceType no commodity Record carries"
    );
    let kind = built.summary().kind(Kind::Commodities);
    assert_eq!(
        kind.map(|kind| (kind.count, kind.skipped, kind.complete)),
        Some((4, 1, true)),
        "counts"
    );
    Ok(())
}

#[test]
fn a_commodity_carries_its_guids_texts_and_display_type() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        catalog["commodities"][0],
        json!({
            "guid": "a1000000-0000-4000-8000-000000000001",
            "className": "gold",
            "recordPath": "libs/foundry/records/entities/commodities/metals/gold",
            "name": {"key": "@items_commodities_gold", "en": "Gold", "de": "Gold"},
            "description": {
                "key": "@items_commodities_gold_desc",
                "en": "A precious metal.",
                "de": "Ein Edelmetall.",
            },
            "typeGuid": "c1000000-0000-4000-8000-000000000001",
            "subtypeGuid": "b1000000-0000-4000-8000-000000000001",
            "displayType": {"key": "@items_commodities_type_metal", "en": "Metal", "de": "Metall"},
        }),
        "the type is the group, the subtype the ResourceType; the description of a placeholder comes from the ResourceType"
    );
    Ok(())
}

#[test]
fn a_commodity_keeps_its_own_description_and_may_lack_a_resource_type() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        catalog["commodities"][1]["description"],
        json!({"key": "@items_commodities_dopple_desc", "en": "A vice.", "de": null}),
        "own description"
    );
    assert_eq!(
        catalog["commodities"][1]["typeGuid"],
        Value::Null,
        "no type"
    );
    Ok(())
}

#[test]
fn blueprints_are_selected_by_prefix_and_type_without_leftovers_ordered_by_guid() -> TestResult {
    let (built, catalog) = build(None)?;
    assert_eq!(
        class_names(&catalog, "blueprints"),
        [
            "BP_CRAFT_behr_lmg_01_mag",
            "BP_CRAFT_core",
            "BP_CRAFT_dangling",
            "BP_CRAFT_weapon",
            "BP_CRAFT_nested",
        ],
        "crafting blueprints incl. $templates, without the dismantle record and the leftover"
    );
    let kind = built.summary().kind(Kind::Blueprints);
    assert_eq!(
        kind.map(|kind| (kind.count, kind.skipped, kind.complete)),
        Some((5, 1, true)),
        "counts"
    );
    Ok(())
}

#[test]
fn a_blueprint_carries_key_category_product_and_craft_time() -> TestResult {
    let (_, catalog) = build(None)?;
    let blueprint = &catalog["blueprints"][1];
    assert_eq!(
        (
            &blueprint["guid"],
            &blueprint["className"],
            &blueprint["recordPath"],
            &blueprint["key"],
        ),
        (
            &json!("12000000-0000-4000-8000-000000000002"),
            &json!("BP_CRAFT_core"),
            &json!(
                "libs/foundry/records/crafting/blueprints/crafting/vehiclegear/powerplant/bp_craft_core"
            ),
            &json!("BP_CRAFT_core"),
        ),
        "identity"
    );
    assert_eq!(
        blueprint["name"],
        json!({"key": "@LOC_PLACEHOLDER", "en": null, "de": null}),
        "placeholder name"
    );
    assert_eq!(
        blueprint["categoryGuid"], "d1000000-0000-4000-8000-000000000001",
        "category"
    );
    assert_eq!(
        blueprint["producedItemGuid"], "e2000000-0000-4000-8000-000000000002",
        "product"
    );
    assert_eq!(
        blueprint["craftTimeSeconds"], 93_784,
        "1 d 2 h 3 min and 3.6 s, rounded"
    );
    Ok(())
}

#[test]
fn ingredients_with_resources_and_items_keep_quantity_quality_and_slot() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        catalog["blueprints"][0]["ingredients"],
        json!([
            {
                "resourceGuid": "b1000000-0000-4000-8000-000000000001", "itemGuid": null,
                "quantityScu": 0.5, "quantityUnits": null, "minQuality": 0,
                "slot": "MAGAZINE", "slotKey": "@crafting_ui_slotname_magazine",
            },
            {
                "resourceGuid": null, "itemGuid": "e2000000-0000-4000-8000-000000000002",
                "quantityScu": null, "quantityUnits": 5, "minQuality": 500,
                "slot": "CORE", "slotKey": null,
            },
            {
                "resourceGuid": "b2000000-0000-4000-8000-000000000002", "itemGuid": null,
                "quantityScu": 0.25, "quantityUnits": null, "minQuality": 300,
                "slot": "CORE", "slotKey": null,
            },
            {
                "resourceGuid": "b2000000-0000-4000-8000-000000000002", "itemGuid": null,
                "quantityScu": 2.0, "quantityUnits": null, "minQuality": 100,
                "slot": "ASPECTS", "slotKey": null,
            },
        ]),
        "selections are flattened depth first, each line with its nearest slot"
    );
    assert_eq!(
        catalog["blueprints"][1]["ingredients"][0]["slot"],
        Value::Null,
        "a cost outside any selection has no slot"
    );
    Ok(())
}

#[test]
fn a_slot_is_inherited_through_unnamed_nested_selections() -> TestResult {
    let (_, catalog) = build(None)?;
    let nested = &catalog["blueprints"][4];
    assert_eq!(nested["className"], "BP_CRAFT_nested", "the blueprint");
    assert_eq!(
        nested["ingredients"][0],
        json!({
            "resourceGuid": "b1000000-0000-4000-8000-000000000001", "itemGuid": null,
            "quantityScu": 3.0, "quantityUnits": null, "minQuality": 200,
            "slot": "OUTER", "slotKey": "@crafting_ui_slotname_outer",
        }),
        "two unnamed selections down, the slot of the named one"
    );
    Ok(())
}

#[test]
fn optional_costs_follow_the_mandatory_ones() -> TestResult {
    let (_, catalog) = build(None)?;
    let ingredients = &catalog["blueprints"][4]["ingredients"];
    assert_eq!(
        ingredients.as_array().map(Vec::len),
        Some(3),
        "one mandatory and two optional lines"
    );
    assert_eq!(
        ingredients[1],
        json!({
            "resourceGuid": "b2000000-0000-4000-8000-000000000002", "itemGuid": null,
            "quantityScu": 0.75, "quantityUnits": null, "minQuality": 400,
            "slot": null, "slotKey": null,
        }),
        "an optional resource outside any selection"
    );
    assert_eq!(
        (&ingredients[2]["itemGuid"], &ingredients[2]["slot"]),
        (
            &json!("e2000000-0000-4000-8000-000000000002"),
            &json!("EXTRA")
        ),
        "an optional item in a selection"
    );
    Ok(())
}

#[test]
fn a_placeholder_name_and_description_fall_back_to_the_resource_type() -> TestResult {
    let (_, catalog) = build(None)?;
    let ember = &catalog["commodities"][2];
    assert_eq!(ember["className"], "ember", "the commodity");
    assert_eq!(
        (&ember["name"], &ember["description"]),
        (
            &json!({"key": "@items_commodities_ember", "en": "Ember", "de": null}),
            &json!({"key": "@items_commodities_ember_desc", "en": "A glowing mineral.", "de": null}),
        ),
        "the texts of the ResourceType"
    );
    Ok(())
}

#[test]
fn a_used_resource_type_without_a_commodity_record_is_a_commodity_of_its_group() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        catalog["commodities"][3],
        json!({
            "guid": "b2000000-0000-4000-8000-000000000002",
            "className": "iron",
            "recordPath": "libs/foundry/records/resourcetypedatabase/resourcetypedatabase",
            "name": {"key": "@items_commodities_iron", "en": null, "de": null},
            "description": {"key": "@items_commodities_iron_desc", "en": null, "de": null},
            "typeGuid": "c1000000-0000-4000-8000-000000000001",
            "subtypeGuid": "b2000000-0000-4000-8000-000000000002",
            "displayType": {"key": null, "en": null, "de": null},
        }),
        "the ResourceType and its group"
    );
    assert!(
        !class_names(&catalog, "commodities").contains(&"copper"),
        "an unused ResourceType stays out"
    );
    Ok(())
}

#[test]
fn every_resource_ingredient_joins_exactly_one_commodity() -> TestResult {
    let (_, catalog) = build(None)?;
    let subtypes: Vec<&Value> = catalog["commodities"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|commodity| &commodity["subtypeGuid"])
                .collect()
        })
        .unwrap_or_default();
    let blueprints = catalog["blueprints"].as_array().ok_or("blueprints")?;
    for ingredient in blueprints
        .iter()
        .flat_map(|blueprint| blueprint["ingredients"].as_array().into_iter().flatten())
        .filter(|ingredient| !ingredient["resourceGuid"].is_null())
    {
        let joined = subtypes
            .iter()
            .filter(|subtype| **subtype == &ingredient["resourceGuid"])
            .count();
        assert_eq!(joined, 1, "{}", ingredient["resourceGuid"]);
    }
    Ok(())
}

#[test]
fn a_product_the_database_lacks_is_null() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        catalog["blueprints"][2]["producedItemGuid"],
        Value::Null,
        "dangling Reference"
    );
    assert_eq!(
        catalog["blueprints"][2]["ingredients"][0]["itemGuid"],
        "e2000000-0000-4000-8000-000000000002",
        "the rest of the blueprint is read"
    );
    Ok(())
}

#[test]
fn a_filter_marks_both_kinds_partial_and_selects_by_record_path() -> TestResult {
    let filter = Filter::new("*/metals/*");
    let (built, catalog) = build(Some(&filter))?;
    assert_eq!(class_names(&catalog, "commodities"), ["gold"], "filtered");
    assert!(class_names(&catalog, "blueprints").is_empty(), "filtered");
    for kind in ["commodities", "blueprints"] {
        assert_eq!(
            catalog["completeness"][kind],
            json!({"status": "partial", "filter": "*/metals/*"}),
            "{kind} is partial"
        );
    }
    assert!(
        built.summary().kinds().iter().all(|kind| !kind.complete),
        "the summary agrees"
    );
    Ok(())
}

#[test]
fn without_a_filter_both_kinds_are_complete() -> TestResult {
    let (_, catalog) = build(None)?;
    for kind in ["commodities", "blueprints"] {
        assert_eq!(
            catalog["completeness"][kind],
            json!({"status": "complete"}),
            "{kind} is complete"
        );
    }
    Ok(())
}

#[test]
fn the_blueprint_kind_selects_crafting_blueprint_records() {
    assert_eq!(
        Kind::Blueprints.rule().type_name,
        "CraftingBlueprintRecord",
        "type"
    );
}
