//! Commodities and blueprints in the Basetool catalog of a real Archive; runs with `--ignored`
//! when `P4KX_DATA_P4K` names one.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, DECOMPRESSED_LIMIT, Kind, build_catalog, catalog_schema};
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MIN_COMMODITIES: usize = 100;
const MIN_BLUEPRINTS: usize = 1_000;
const MIN_LINKED: f64 = 0.95;

fn list<'c>(catalog: &'c Value, kind: &str) -> Result<&'c Vec<Value>, String> {
    catalog[kind]
        .as_array()
        .ok_or_else(|| format!("the {kind} array"))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a share of counts far below 2^52"
)]
fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        1.0
    } else {
        part as f64 / whole as f64
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn builds_commodities_and_blueprints_of_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let validator = jsonschema::validator_for(&catalog_schema())?;
    let errors: Vec<String> = validator
        .iter_errors(&catalog)
        .take(5)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "valid against the schema: {errors:?}");
    report(&built);
    assert!(
        built.summary().size() < DECOMPRESSED_LIMIT,
        "under the decompressed limit"
    );
    for (kind, min) in [
        (Kind::Commodities, MIN_COMMODITIES),
        (Kind::Blueprints, MIN_BLUEPRINTS),
    ] {
        let counts = built.summary().kind(kind).ok_or("the kind")?;
        assert!(counts.count >= min, "plausible {kind} count: {counts:?}");
        assert!(counts.complete, "{kind} complete without a Filter");
        let guids: Vec<&str> = list(&catalog, kind.name())?
            .iter()
            .filter_map(|entity| entity["guid"].as_str())
            .collect();
        assert!(guids.is_sorted(), "{kind} ordered by GUID");
    }
    check_commodities(&catalog)?;
    check_blueprints(&catalog)
}

fn check_commodities(catalog: &Value) -> TestResult {
    let commodities = list(catalog, "commodities")?;
    let gold = commodities
        .iter()
        .find(|commodity| {
            commodity["className"]
                .as_str()
                .is_some_and(|name| name.eq_ignore_ascii_case("gold"))
        })
        .ok_or("the commodity Gold")?;
    assert_eq!(
        gold.pointer("/name/en"),
        Some(&Value::from("Gold")),
        "English name"
    );
    assert!(gold["typeGuid"].is_string(), "type: the group");
    assert!(gold["subtypeGuid"].is_string(), "subtype: the ResourceType");
    let described = commodities
        .iter()
        .filter(|commodity| {
            commodity
                .pointer("/description/en")
                .is_some_and(Value::is_string)
        })
        .count();
    assert!(
        ratio(described, commodities.len()) > 0.9,
        "most commodities have a description: {described} of {}",
        commodities.len()
    );
    Ok(())
}

fn check_blueprints(catalog: &Value) -> TestResult {
    let blueprints = list(catalog, "blueprints")?;
    let mut resources: HashMap<&str, usize> = HashMap::new();
    for subtype in list(catalog, "commodities")?
        .iter()
        .filter_map(|commodity| commodity["subtypeGuid"].as_str())
    {
        *resources.entry(subtype).or_default() += 1;
    }
    let items: HashSet<&str> = list(catalog, "items")?
        .iter()
        .filter_map(|item| item["guid"].as_str())
        .collect();
    let produced: Vec<&str> = blueprints
        .iter()
        .filter_map(|blueprint| blueprint["producedItemGuid"].as_str())
        .collect();
    let known = produced.iter().filter(|guid| items.contains(*guid)).count();
    assert!(
        ratio(known, produced.len()) >= MIN_LINKED,
        "{known} of {} produced items are catalog items",
        produced.len()
    );
    check_ingredients(catalog, &resources)?;
    check_craft_times(blueprints);
    Ok(())
}

fn check_ingredients(catalog: &Value, resources: &HashMap<&str, usize>) -> TestResult {
    let blueprints = list(catalog, "blueprints")?;
    let lines: Vec<&Value> = blueprints
        .iter()
        .filter_map(|blueprint| blueprint["ingredients"].as_array())
        .flatten()
        .collect();
    assert!(!lines.is_empty(), "ingredient lines");
    let resource_lines: HashSet<&str> = lines
        .iter()
        .filter_map(|line| line["resourceGuid"].as_str())
        .collect();
    let unjoined: Vec<&&str> = resource_lines
        .iter()
        .filter(|guid| resources.get(**guid) != Some(&1))
        .collect();
    assert!(
        !resource_lines.is_empty() && unjoined.is_empty(),
        "every resource GUID joins exactly one commodity: {unjoined:?}"
    );
    assert!(
        lines.iter().any(|line| line["itemGuid"].is_string()),
        "some ingredient is an item"
    );
    assert!(
        lines
            .iter()
            .all(|line| line["resourceGuid"].is_string() != line["itemGuid"].is_string()),
        "a line is a resource or an item"
    );
    assert!(
        lines
            .iter()
            .any(|line| line["minQuality"].as_i64().is_some_and(|q| q > 0)),
        "some ingredient has a minimum quality"
    );
    Ok(())
}

fn check_craft_times(blueprints: &[Value]) {
    let timed = blueprints
        .iter()
        .filter(|blueprint| {
            blueprint["craftTimeSeconds"]
                .as_i64()
                .is_some_and(|s| s > 0)
        })
        .count();
    assert!(
        ratio(timed, blueprints.len()) > 0.9,
        "most blueprints have a craft time: {timed} of {}",
        blueprints.len()
    );
}

#[expect(
    clippy::print_stdout,
    reason = "the sizes and counts are the measurement the upload limit is sized from"
)]
fn report(built: &p4kx_export::BuiltCatalog) {
    println!("catalog size: {} bytes", built.summary().size());
    for kind in built.summary().kinds() {
        println!("{kind:?}");
    }
    for text in built.summary().texts() {
        println!("{text:?}");
    }
}
