//! English and German names and descriptions in the Basetool catalog, through the public API.

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{CatalogOptions, Language, TextSummary, build_catalog, build_catalog_from};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestStruct, TestType, TestValue,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const EXPORTED_AT: Duration = Duration::from_secs(1_792_000_000);

fn manufacturer(guid: u128, class: &str, name: &str, description: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "SCItemManufacturer",
        format!("SCItemManufacturer.{class}"),
        format!("libs/foundry/records/scitemmanufacturer/{class}.xml"),
    )
    .value("Code", TestValue::text(class))
    .value(
        "Localization",
        TestValue::Struct(
            TestInstance::new("SCItemLocalization")
                .value("Name", TestValue::text(name))
                .value("Description", TestValue::text(description)),
        ),
    )
}

fn database() -> Vec<u8> {
    DataForgeBuilder::new()
        .version(8)
        .structure(
            TestStruct::new("SCItemLocalization")
                .field("Name", TestType::Locale)
                .field("Description", TestType::Locale),
        )
        .structure(
            TestStruct::new("SCItemManufacturer")
                .field("Code", TestType::String)
                .field(
                    "Localization",
                    TestType::Class("SCItemLocalization".to_owned()),
                ),
        )
        .record(manufacturer(
            0x1000_0000_0000_4000_8000_0000_0000_0001,
            "ACAS",
            "@manufacturer_NameACAS",
            "@manufacturer_DescACAS",
        ))
        .record(manufacturer(
            0x2000_0000_0000_4000_8000_0000_0000_0002,
            "DRAK",
            "@manufacturer_NameDRAK",
            "",
        ))
        .record(manufacturer(
            0x3000_0000_0000_4000_8000_0000_0000_0003,
            "MISC",
            "@loc_placeholder",
            "@manufacturer_DescMISC",
        ))
        .build()
}

fn english() -> TestEntry {
    TestEntry::zstd(
        Language::English.global_ini_path(),
        GlobalIniBuilder::new()
            .text("manufacturer_NameACAS", "Ace Astrogation")
            .text("manufacturer_DescACAS", "Power plants.")
            .text("manufacturer_NameDRAK", "Drake Interplanetary")
            .text("LOC_PLACEHOLDER", "<= PLACEHOLDER =>")
            .text("manufacturer_DescMISC", "Line one\\nLine two\rkept")
            .build(),
    )
}

fn german() -> TestEntry {
    TestEntry::zstd(
        Language::German.global_ini_path(),
        GlobalIniBuilder::new()
            .text("manufacturer_NameACAS", "Ace Astrogation")
            .text("manufacturer_DescACAS", "Kraftwerke.")
            .text("manufacturer_DescMISC", "")
            .build(),
    )
}

fn catalog(entries: Vec<TestEntry>) -> Result<(Value, Vec<TextSummary>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    entries
        .into_iter()
        .fold(
            ArchiveBuilder::new().entry(TestEntry::zstd("Data\\Game2.dcb", database())),
            ArchiveBuilder::entry,
        )
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let built = build_catalog(&archive, &CatalogOptions::new(UNIX_EPOCH + EXPORTED_AT))?;
    let json = serde_json::from_slice(built.json())?;
    Ok((json, built.summary().texts().to_vec()))
}

fn texts(catalog: &Value, index: usize) -> (Value, Value) {
    let entity = catalog["manufacturers"].get(index).unwrap_or(&Value::Null);
    (entity["name"].clone(), entity["description"].clone())
}

#[test]
fn names_and_descriptions_hold_the_key_and_both_resolved_texts() -> TestResult {
    let (catalog, _) = catalog(vec![english(), german()])?;
    assert_eq!(
        texts(&catalog, 0),
        (
            json!({"key": "@manufacturer_NameACAS", "en": "Ace Astrogation", "de": "Ace Astrogation"}),
            json!({"key": "@manufacturer_DescACAS", "en": "Power plants.", "de": "Kraftwerke."}),
        ),
        "ACAS in both languages"
    );
    Ok(())
}

#[test]
fn a_missing_key_leaves_its_text_null_keeps_the_key_and_is_counted() -> TestResult {
    let (catalog, summary) = catalog(vec![english(), german()])?;
    assert_eq!(
        texts(&catalog, 1),
        (
            json!({"key": "@manufacturer_NameDRAK", "en": "Drake Interplanetary", "de": null}),
            json!({"key": null, "en": null, "de": null}),
        ),
        "DRAK has no German name and no description"
    );
    assert_eq!(
        summary,
        [
            TextSummary {
                language: Language::English,
                available: true,
                resolved: 4,
                unresolved: 0,
                placeholder: 1,
            },
            TextSummary {
                language: Language::German,
                available: true,
                resolved: 2,
                unresolved: 2,
                placeholder: 1,
            },
        ],
        "four keys and a placeholder; one key not in German, one empty"
    );
    Ok(())
}

#[test]
fn a_missing_language_leaves_its_texts_null_and_counts_every_key() -> TestResult {
    let (catalog, summary) = catalog(vec![english()])?;
    assert_eq!(
        texts(&catalog, 0).0,
        json!({"key": "@manufacturer_NameACAS", "en": "Ace Astrogation", "de": null}),
        "English only"
    );
    assert_eq!(
        summary.get(1),
        Some(&TextSummary {
            language: Language::German,
            available: false,
            resolved: 0,
            unresolved: 4,
            placeholder: 1,
        }),
        "German is missing"
    );
    Ok(())
}

#[test]
fn a_placeholder_key_keeps_the_key_has_no_texts_and_is_counted_apart() -> TestResult {
    let (catalog, summary) = catalog(vec![english(), german()])?;
    assert_eq!(
        texts(&catalog, 2).0,
        json!({"key": "@loc_placeholder", "en": null, "de": null}),
        "LOC_PLACEHOLDER=<= PLACEHOLDER => is not a name"
    );
    assert!(
        summary.iter().all(|texts| texts.placeholder == 1),
        "one placeholder in every language: {summary:?}"
    );
    Ok(())
}

#[test]
fn an_empty_text_is_null_and_unresolved() -> TestResult {
    let (catalog, _) = catalog(vec![english(), german()])?;
    assert_eq!(
        texts(&catalog, 2).1["de"],
        Value::Null,
        "manufacturer_DescMISC= in German"
    );
    Ok(())
}

#[test]
fn the_line_break_escape_becomes_a_line_feed_and_a_lone_cr_stays() -> TestResult {
    let (catalog, _) = catalog(vec![english(), german()])?;
    assert_eq!(
        texts(&catalog, 2).1["en"],
        "Line one\nLine two\rkept",
        "\\n decoded in a CRLF file, the lone CR kept"
    );
    Ok(())
}

#[test]
fn a_catalog_from_a_read_entry_equals_one_read_from_the_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database()))
        .entry(english())
        .entry(german())
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let options = CatalogOptions::new(UNIX_EPOCH + EXPORTED_AT);

    let read = build_catalog(&archive, &options)?;
    let entry = DataForgeEntry::read(&archive)?;
    let from = build_catalog_from(&archive, &entry, &options)?;

    assert_eq!(from.json(), read.json(), "identical bytes");
    assert_eq!(from.summary(), read.summary(), "identical summary");
    Ok(())
}
