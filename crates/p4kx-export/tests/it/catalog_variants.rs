//! The variant groups of ships in the Basetool catalog, through the public API.

use crate::variants_fixture;

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{BuiltCatalog, CatalogOptions, VariantCounts, build_catalog};
use p4kx_extraction::Archive;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const EXPORTED_AT: Duration = Duration::from_secs(1_792_000_000);

fn build() -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = Archive::open_with_key(
        variants_fixture::write_archive(dir.path())?,
        Some(p4kx_testkit::test_key()),
    )?;
    let built = build_catalog(&archive, &CatalogOptions::new(UNIX_EPOCH + EXPORTED_AT))?;
    let json = serde_json::from_slice(built.json())?;
    Ok((built, json))
}

fn entry<'c>(catalog: &'c Value, class: &str) -> Result<&'c Value, String> {
    catalog["ships"]
        .as_array()
        .and_then(|ships| ships.iter().find(|ship| ship["className"] == class))
        .ok_or_else(|| format!("ship {class} missing"))
}

fn base_class(catalog: &Value, class: &str) -> Result<Option<String>, String> {
    let base = &entry(catalog, class)?["variantOf"];
    if base.is_null() {
        return Ok(None);
    }
    catalog["ships"]
        .as_array()
        .and_then(|ships| ships.iter().find(|ship| ship["guid"] == *base))
        .and_then(|ship| ship["className"].as_str())
        .map(|name| Some(name.to_owned()))
        .ok_or_else(|| format!("variantOf of {class} resolves to no ship"))
}

#[test]
fn ships_with_the_same_definition_implementation_and_armour_are_variants_of_the_smallest_class_name()
-> TestResult {
    let (_, catalog) = build()?;
    for (class, base) in [
        ("ACAS_Alpha", None),
        ("ACAS_Alpha_Edition", Some("ACAS_Alpha")),
        ("ACAS_Alpha_Zulu", Some("ACAS_Alpha")),
    ] {
        assert_eq!(base_class(&catalog, class)?.as_deref(), base, "{class}");
    }
    Ok(())
}

#[test]
fn a_modification_that_changes_the_implementation_starts_a_group_of_its_own() -> TestResult {
    let (_, catalog) = build()?;
    for (class, base) in [
        ("ACAS_Beta", None),
        ("ACAS_Beta_Edition", Some("ACAS_Beta")),
    ] {
        assert_eq!(base_class(&catalog, class)?.as_deref(), base, "{class}");
    }
    Ok(())
}

#[test]
fn a_ship_that_differs_in_armour_definition_or_modification_is_not_a_variant() -> TestResult {
    let (_, catalog) = build()?;
    for class in ["ACAS_Armoured", "ACAS_Other", "ACAS_Solo"] {
        assert_eq!(base_class(&catalog, class)?, None, "{class}");
    }
    Ok(())
}

#[test]
fn a_ship_without_a_readable_implementation_is_ungrouped() -> TestResult {
    let (_, catalog) = build()?;
    for class in ["ACAS_Lonely", "ACAS_Broken", "ACAS_Missing"] {
        assert_eq!(base_class(&catalog, class)?, None, "{class}");
    }
    Ok(())
}

#[test]
fn every_ship_carries_the_field_and_it_is_a_guid_or_null() -> TestResult {
    let (_, catalog) = build()?;
    let ships = catalog["ships"].as_array().ok_or("ships")?;
    assert_eq!(ships.len(), 11);
    for ship in ships {
        let value = ship.get("variantOf").ok_or("variantOf missing")?;
        assert!(value.is_null() || value.is_string(), "{}", ship["className"]);
    }
    Ok(())
}

#[test]
fn the_summary_counts_groups_and_variants() -> TestResult {
    let (built, _) = build()?;
    assert_eq!(
        built.summary().variants().ships,
        VariantCounts {
            groups: 2,
            variants: 3
        }
    );
    Ok(())
}
