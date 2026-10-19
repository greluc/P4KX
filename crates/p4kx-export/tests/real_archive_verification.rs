//! The verification of M3 over real Archives: the whole catalog against the checked-in schema,
//! plausible counts, no development leftovers, resolved texts and the upload limit; runs with
//! `--ignored` when `P4KX_DATA_P4K` (and for the second Channel `P4KX_DATA_P4K_B`) names one.

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

use p4kx_export::{
    BuiltCatalog, CatalogOptions, EXCLUDED_TOKENS, Kind, Language, PLACEHOLDER_KEYS, UPLOAD_LIMIT,
    build_catalog, catalog_schema,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_b_path, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MINIMUM: [(Kind, usize); 5] = [
    (Kind::Manufacturers, 500),
    (Kind::Items, 10_000),
    (Kind::Ships, 200),
    (Kind::Commodities, 100),
    (Kind::Blueprints, 1_000),
];
const MIN_ENGLISH_SHARE: f64 = 0.95;
const MIN_GERMAN_SHARE: f64 = 0.5;
const MIN_HEADROOM_FACTOR: u64 = 2;

fn built(path: &Path) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let archive = Archive::open(path)?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog = serde_json::from_slice(built.json())?;
    Ok((built, catalog))
}

fn list(catalog: &Value, kind: Kind) -> Result<&Vec<Value>, String> {
    catalog[kind.name()]
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

fn is_placeholder(key: &str) -> bool {
    PLACEHOLDER_KEYS.iter().any(|placeholder| {
        placeholder
            .trim_start_matches('@')
            .eq_ignore_ascii_case(key.trim_start_matches('@'))
    })
}

fn has_excluded_token(text: &str) -> Option<&'static str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .find_map(|word| {
            EXCLUDED_TOKENS
                .into_iter()
                .find(|token| word.eq_ignore_ascii_case(token))
        })
}

fn verify_catalog(path: &Path) -> Result<Value, Box<dyn Error>> {
    let (built, catalog) = built(path)?;
    let validator = jsonschema::validator_for(&catalog_schema())?;
    let errors: Vec<String> = validator
        .iter_errors(&catalog)
        .take(5)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "valid against the schema: {errors:?}");
    report(path, &built);

    for (kind, minimum) in MINIMUM {
        let counts = built.summary().kind(kind).ok_or("the kind")?;
        assert!(
            counts.count >= minimum,
            "plausible {kind} count: {counts:?}"
        );
        assert!(counts.complete, "{kind} complete without a Filter");
        let entities = list(&catalog, kind)?;
        assert_eq!(entities.len(), counts.count, "{kind} written");
        let guids: Vec<&str> = entities.iter().filter_map(|e| e["guid"].as_str()).collect();
        assert_eq!(guids.len(), entities.len(), "{kind} all have a GUID");
        assert!(guids.is_sorted(), "{kind} ordered by GUID");
    }
    assert!(
        built.summary().size() < UPLOAD_LIMIT / MIN_HEADROOM_FACTOR,
        "the limit holds at least {MIN_HEADROOM_FACTOR} times the catalog"
    );
    check_leftovers(&catalog)?;
    check_texts(&built, &catalog)?;
    Ok(catalog)
}

fn check_leftovers(catalog: &Value) -> TestResult {
    for kind in Kind::ALL {
        for entity in list(catalog, kind)? {
            for field in ["recordPath", "className"] {
                let text = entity[field].as_str().unwrap_or_default();
                assert_eq!(
                    has_excluded_token(text),
                    None,
                    "development leftover in {kind}: {text}"
                );
            }
        }
    }
    Ok(())
}

fn check_texts(built: &BuiltCatalog, catalog: &Value) -> TestResult {
    let mut keyed = 0_usize;
    let mut english = 0_usize;
    let mut german = 0_usize;
    for kind in Kind::ALL {
        let mut kind_keyed = 0_usize;
        let mut kind_english = 0_usize;
        let mut kind_german = 0_usize;
        for entity in list(catalog, kind)? {
            for field in ["name", "description"] {
                let Some(text) = entity.get(field) else {
                    continue;
                };
                let Some(key) = text.get("key").and_then(Value::as_str) else {
                    continue;
                };
                if is_placeholder(key) {
                    continue;
                }
                kind_keyed += 1;
                kind_english += usize::from(text.get("en").is_some_and(Value::is_string));
                kind_german += usize::from(text.get("de").is_some_and(Value::is_string));
            }
        }
        print_line(&format!(
            "{kind}: {kind_keyed} keyed texts, {} missing in en, {} missing in de",
            kind_keyed - kind_english,
            kind_keyed - kind_german
        ));
        keyed += kind_keyed;
        english += kind_english;
        german += kind_german;
    }
    let (english_share, german_share) = (ratio(english, keyed), ratio(german, keyed));
    print_line(&format!(
        "all kinds: {keyed} keyed texts, en {english} ({english_share:.3}), de {german} ({german_share:.3})"
    ));
    assert!(
        english_share >= MIN_ENGLISH_SHARE,
        "most texts resolve in en"
    );
    assert!(german_share >= MIN_GERMAN_SHARE, "most texts resolve in de");
    for language in Language::ALL {
        let texts = built.summary().text(language).ok_or("a text summary")?;
        assert!(texts.available, "the Archive holds {language}");
        assert!(
            texts.resolved > texts.unresolved,
            "most {language} texts resolve: {texts:?}"
        );
    }
    Ok(())
}

fn check_links(catalog: &Value) -> TestResult {
    let guids = |kind: Kind| -> Result<HashSet<&str>, String> {
        Ok(list(catalog, kind)?
            .iter()
            .filter_map(|e| e["guid"].as_str())
            .collect())
    };
    let manufacturers = guids(Kind::Manufacturers)?;
    let producible: HashSet<&str> = guids(Kind::Items)?
        .union(&guids(Kind::Ships)?)
        .copied()
        .collect();
    let items = guids(Kind::Items)?;
    let resources: HashSet<&str> = list(catalog, Kind::Commodities)?
        .iter()
        .filter_map(|e| e["subtypeGuid"].as_str())
        .collect();
    for kind in [Kind::Items, Kind::Ships] {
        for entity in list(catalog, kind)? {
            if let Some(guid) = entity["manufacturerGuid"].as_str() {
                assert!(manufacturers.contains(guid), "{kind} manufacturer {guid}");
            }
        }
    }
    for blueprint in list(catalog, Kind::Blueprints)? {
        if let Some(guid) = blueprint["producedItemGuid"].as_str() {
            assert!(producible.contains(guid), "produced entity {guid}");
        }
        for ingredient in blueprint["ingredients"].as_array().into_iter().flatten() {
            if let Some(guid) = ingredient["resourceGuid"].as_str() {
                assert!(resources.contains(guid), "resource {guid}");
            }
            if let Some(guid) = ingredient["itemGuid"].as_str() {
                assert!(items.contains(guid), "ingredient item {guid}");
            }
        }
    }
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the measurements are the result of the verification"
)]
fn print_line(line: &str) {
    println!("{line}");
}

fn report(path: &Path, built: &BuiltCatalog) {
    let summary = built.summary();
    print_line(&format!(
        "{}: catalog size {} bytes, upload limit {UPLOAD_LIMIT} bytes",
        path.display(),
        summary.size()
    ));
    for kind in summary.kinds() {
        print_line(&format!("{kind:?}"));
    }
    print_line(&format!("{:?}", summary.item_classes()));
    for text in summary.texts() {
        print_line(&format!("{text:?}"));
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_catalog_of_a_real_archive_is_complete_plausible_and_within_the_limit() -> TestResult {
    let path = real_archive_path()?;
    check_links(&verify_catalog(&path)?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_catalog_of_a_second_real_archive_is_complete_plausible_and_within_the_limit() -> TestResult {
    let path = real_archive_b_path()?;
    check_links(&verify_catalog(&path)?)
}
