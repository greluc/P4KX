// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Quality quantizations in the Basetool catalog, through the public API.

use crate::quantization_fixture::{
    ANOTHER, GOLD, IRON, NO_BANDS, ORE_GOLD, ORE_IRON, ORPHAN, REVERSED, SHARED_GOLD,
    TWO_PROPERTIES, write_archive,
};

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{
    BuiltCatalog, CatalogOptions, CraftingWarning, Filter, Kind, build_catalog, read_crafting,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn build(filter: Option<&Filter>) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let mut options = CatalogOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000));
    options.filter = filter;
    let built = build_catalog(&archive, &options)?;
    let json = serde_json::from_slice(built.json())?;
    Ok((built, json))
}

fn guid(value: u128) -> String {
    let text = format!("{value:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &text[0..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..32]
    )
}

fn entries(catalog: &Value) -> Vec<&Value> {
    catalog["qualityQuantizations"]
        .as_array()
        .into_iter()
        .flatten()
        .collect()
}

fn class_names(catalog: &Value) -> Vec<&str> {
    entries(catalog)
        .into_iter()
        .filter_map(|entry| entry["className"].as_str())
        .collect()
}

#[test]
fn a_quantization_carries_its_bands_and_the_resource_types_that_link_to_it() -> TestResult {
    let (_, catalog) = build(None)?;
    let gold = entries(&catalog)
        .into_iter()
        .find(|entry| entry["guid"] == guid(GOLD).as_str())
        .ok_or("the gold quantization")?;
    assert_eq!(
        *gold,
        json!({
            "guid": guid(GOLD),
            "className": "Quantization_Gold",
            "recordPath": "libs/foundry/records/crafting/qualityquantization/quantization_gold",
            "bands": [
                {"start": 0, "end": 399, "mappedValue": 264},
                {"start": 400, "end": 999, "mappedValue": 553},
                {"start": 1000, "end": 1000, "mappedValue": 1000},
            ],
            "resourceTypeGuids": [guid(ORE_GOLD), guid(SHARED_GOLD), guid(TWO_PROPERTIES)],
        }),
        "linked once per ResourceType, however many properties link, ordered by GUID"
    );
    Ok(())
}

#[test]
fn the_entries_are_ordered_by_guid_and_a_leftover_is_counted_as_skipped() -> TestResult {
    let (built, catalog) = build(None)?;
    assert_eq!(
        class_names(&catalog),
        [
            "Quantization_Gold",
            "Quantization_Iron",
            "Quantization_Orphan",
            "Quantization_Another"
        ]
    );
    let iron = entries(&catalog)
        .into_iter()
        .find(|entry| entry["guid"] == guid(IRON).as_str())
        .ok_or("the iron quantization")?;
    assert_eq!(iron["resourceTypeGuids"], json!([guid(ORE_IRON)]));
    let kind = built.summary().kind(Kind::QualityQuantizations);
    assert_eq!(
        kind.map(|kind| (kind.count, kind.skipped, kind.complete)),
        Some((4, 1, true)),
        "counts"
    );
    Ok(())
}

#[test]
fn an_unlinked_quantization_is_kept_with_no_resource_types_and_reported() -> TestResult {
    let (built, catalog) = build(None)?;
    for unlinked in [ORPHAN, ANOTHER] {
        let entry = entries(&catalog)
            .into_iter()
            .find(|entry| entry["guid"] == guid(unlinked).as_str())
            .ok_or("the unlinked quantization")?;
        assert_eq!(entry["resourceTypeGuids"], json!([]));
        assert_eq!(entry["bands"].as_array().map(Vec::len), Some(3));
    }
    assert_eq!(
        built.summary().warnings(),
        [CraftingWarning::UnlinkedQualityQuantizations {
            count: 2,
            details: vec!["Quantization_Another".into(), "Quantization_Orphan".into()],
        }]
    );
    Ok(())
}

#[test]
fn a_malformed_quantization_is_a_failure_and_left_out() -> TestResult {
    let (built, catalog) = build(None)?;
    assert!(
        !class_names(&catalog).contains(&"Quantization_NoBands")
            && !class_names(&catalog).contains(&"Quantization_Reversed"),
        "left out"
    );
    let failures: Vec<(String, &str)> = built
        .summary()
        .failures()
        .iter()
        .map(|failure| (failure.guid.to_string(), failure.reason.as_str()))
        .collect();
    assert_eq!(
        failures,
        [
            (guid(NO_BANDS), "the Record holds no bands"),
            (guid(REVERSED), "bands[1] ends before it starts"),
        ]
    );
    assert!(!built.summary().is_complete());
    Ok(())
}

#[test]
fn a_filter_selects_by_record_path_but_the_links_stay_complete() -> TestResult {
    let filter = Filter::new("*/quantization_gold");
    let (built, catalog) = build(Some(&filter))?;
    assert_eq!(class_names(&catalog), ["Quantization_Gold"]);
    assert_eq!(
        entries(&catalog)[0]["resourceTypeGuids"]
            .as_array()
            .map(Vec::len),
        Some(3),
        "the ResourceType Records are outside the Filter and still link"
    );
    assert_eq!(
        catalog["completeness"]["qualityQuantizations"],
        json!({"status": "partial", "filter": "*/quantization_gold"})
    );
    assert!(
        built.summary().warnings().is_empty(),
        "the unlinked Records are filtered out"
    );
    Ok(())
}

#[test]
fn the_library_reads_the_quantizations_with_their_links() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let crafting = read_crafting(&forge, None, None)?;
    let names: Vec<(&str, usize, usize)> = crafting
        .quality_quantizations
        .iter()
        .map(|quantization| {
            (
                quantization.class_name,
                quantization.bands.len(),
                quantization.resource_types.len(),
            )
        })
        .collect();
    assert_eq!(names.len(), 5, "the leftover is read, the malformed are not");
    assert!(names.contains(&("Quantization_Gold", 3, 3)));
    assert_eq!(crafting.failures.len(), 2);
    Ok(())
}
