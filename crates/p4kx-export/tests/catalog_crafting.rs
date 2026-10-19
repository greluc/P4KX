//! Commodities and blueprints of the Basetool catalog, through the public API.

mod crafting_fixture;

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{BuiltCatalog, CatalogOptions, Filter, Kind, build_catalog};
use p4kx_extraction::Archive;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn build(filter: Option<&Filter>) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = Archive::open(crafting_fixture::write_archive(dir.path())?)?;
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
        ["gold", "dopple"],
        "under entities/commodities, without the test commodity and the carryable"
    );
    let kind = built.summary().kind(Kind::Commodities);
    assert_eq!(
        kind.map(|kind| (kind.count, kind.skipped, kind.complete)),
        Some((2, 1, true)),
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
            "typeGuid": "b1000000-0000-4000-8000-000000000001",
            "subtypeGuid": "c1000000-0000-4000-8000-000000000001",
            "displayType": {"key": "@items_commodities_type_metal", "en": "Metal", "de": "Metall"},
        }),
        "the description of a placeholder comes from the ResourceType"
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
        ],
        "crafting blueprints incl. $templates, without the dismantle record and the leftover"
    );
    let kind = built.summary().kind(Kind::Blueprints);
    assert_eq!(
        kind.map(|kind| (kind.count, kind.skipped, kind.complete)),
        Some((4, 1, true)),
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
fn the_kinds_list_five_selections() {
    assert_eq!(
        Kind::ALL.map(Kind::name),
        [
            "manufacturers",
            "items",
            "ships",
            "commodities",
            "blueprints"
        ],
        "order of the catalog"
    );
    assert_eq!(
        Kind::Blueprints.rule().type_name,
        "CraftingBlueprintRecord",
        "type"
    );
}
