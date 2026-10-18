//! `p4kx catalog` end to end, over a synthetic DataForge and English and German `global.ini`
//! files.

use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestStruct, TestType, TestValue, write_build_manifest,
};

type TestResult = Result<(), Box<dyn Error>>;

const EXPORTED_AT: &str = "1792000000";
const MODIFIED: Duration = Duration::from_hours(497_500);

fn manufacturer(guid: u128, class: &str, path: &str, code: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "SCItemManufacturer",
        format!("SCItemManufacturer.{class}"),
        format!("libs/foundry/records/scitemmanufacturer/{path}.xml"),
    )
    .value("Code", TestValue::text(code))
    .value(
        "Localization",
        TestValue::Struct(
            TestInstance::new("SCItemLocalization")
                .value(
                    "Name",
                    TestValue::text(format!("@manufacturer_Name{class}")),
                )
                .value(
                    "Description",
                    TestValue::text(format!("@manufacturer_Desc{class}")),
                ),
        ),
    )
}

fn database() -> DataForgeBuilder {
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
            0xcfc0_122d_5275_415a_a656_3dcb_3346_feb5,
            "ACAS",
            "acas",
            "ACAS",
        ))
        .record(manufacturer(
            0x0a1b_0000_0000_4000_8000_0000_0000_0001,
            "AEGS",
            "scitemmanufacturer.aegs",
            "AEGS",
        ))
        .record(manufacturer(
            0x5555_0000_0000_4000_8000_0000_0000_0002,
            "Paint_Test_01",
            "paint_test_01",
            "",
        ))
}

fn english() -> TestEntry {
    TestEntry::zstd(
        "Data\\Localization\\english\\global.ini",
        GlobalIniBuilder::new()
            .text("manufacturer_NameACAS", "Ace Astrogation")
            .text("manufacturer_DescACAS", "Power plants.")
            .text("manufacturer_NameAEGS,P", "Aegis Dynamics")
            .build(),
    )
}

fn german() -> TestEntry {
    TestEntry::zstd(
        "Data\\Localization\\german_(germany)\\global.ini",
        GlobalIniBuilder::new()
            .text("manufacturer_NameACAS", "Ace Astrogation")
            .text("manufacturer_DescACAS", "Kraftwerke.")
            .build(),
    )
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    write_archive_with(dir, &[english(), german()])
}

fn write_archive_with(dir: &Path, texts: &[TestEntry]) -> Result<PathBuf, Box<dyn Error>> {
    let channel = dir.join("LIVE");
    fs::create_dir(&channel)?;
    write_build_manifest(
        &channel,
        "sc-alpha-4.4.0",
        "4.4.162.12345",
        "10012345",
        "0123456789abcdef",
    )?;
    let path = channel.join("Data.p4k");
    texts
        .iter()
        .cloned()
        .fold(
            ArchiveBuilder::new()
                .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted()),
            ArchiveBuilder::entry,
        )
        .write_to(&path)?;
    File::options()
        .write(true)
        .open(&path)?
        .set_modified(UNIX_EPOCH + MODIFIED)?;
    Ok(path)
}

fn p4kx(args: &[&str]) -> Result<Command, Box<dyn Error>> {
    let mut command = Command::cargo_bin("p4kx")?;
    command.args(args).env("SOURCE_DATE_EPOCH", EXPORTED_AT);
    Ok(command)
}

fn catalog(archive: &Path, out: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = p4kx(&["catalog"])?
        .arg(archive)
        .arg("--out")
        .arg(out)
        .args(args)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    let dir = archive
        .parent()
        .and_then(Path::parent)
        .ok_or("the Archive has a folder")?
        .display()
        .to_string();
    Ok(String::from_utf8(output.stdout)?
        .replace(&dir.replace('\\', "\\\\"), "[DIR]")
        .replace(&dir, "[DIR]")
        .replace("\\\\", "/")
        .replace('\\', "/"))
}

fn printed_schema() -> Result<String, Box<dyn Error>> {
    let output = p4kx(&["catalog", "schema"])?.output()?;
    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    Ok(String::from_utf8(output.stdout)?)
}

fn redact_version(text: &str) -> String {
    text.replace(
        &format!("\"toolVersion\":\"{}\"", env!("CARGO_PKG_VERSION")),
        "\"toolVersion\":\"[VERSION]\"",
    )
}

#[test]
fn catalog_schema_prints_the_checked_in_schema() -> TestResult {
    let checked_in = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/basetool-catalog.schema.json"),
    )?;
    assert!(
        printed_schema()? == checked_in,
        "the printed schema is the checked-in one"
    );
    Ok(())
}

#[test]
fn catalog_writes_a_catalog_that_the_schema_accepts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("basetool-catalog.json");

    let summary = catalog(&archive, &out, &[])?;

    let text = fs::read_to_string(&out)?;
    let schema: serde_json::Value = serde_json::from_str(&printed_schema()?)?;
    let validator = jsonschema::validator_for(&schema)?;
    let instance: serde_json::Value = serde_json::from_str(&text)?;
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "valid against the schema: {errors:?}");
    insta::assert_snapshot!("catalog_file", redact_version(&text));
    insta::assert_snapshot!("catalog_summary", summary);
    Ok(())
}

#[test]
fn catalog_with_a_filter_marks_the_kind_partial_and_prints_json() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("partial.json");

    let summary = catalog(&archive, &out, &["--filter", "*/acas", "--json"])?;

    let catalog: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out)?)?;
    assert_eq!(
        catalog["completeness"]["manufacturers"],
        serde_json::json!({"status": "partial", "filter": "*/acas"}),
        "partial with the Filter"
    );
    insta::assert_snapshot!("catalog_filter_json", summary);
    Ok(())
}

#[test]
fn two_catalog_runs_write_identical_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let first = dir.path().join("first.json");
    let second = dir.path().join("second.json");
    catalog(&archive, &first, &[])?;
    catalog(&archive, &second, &[])?;
    assert!(fs::read(first)? == fs::read(second)?, "byte-identical");
    Ok(())
}

#[test]
fn catalog_without_an_output_file_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let output = p4kx(&["catalog"])?.arg(&archive).output()?;
    assert_eq!(output.status.code(), Some(2), "exit code: {output:?}");
    Ok(())
}

#[test]
fn catalog_without_a_german_global_ini_leaves_german_null_and_counts_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive_with(dir.path(), &[english()])?;
    let out = dir.path().join("english-only.json");

    let summary = catalog(&archive, &out, &[])?;

    let catalog: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out)?)?;
    assert_eq!(
        catalog["manufacturers"][1]["name"],
        serde_json::json!({"key": "@manufacturer_NameACAS", "en": "Ace Astrogation", "de": null}),
        "English only"
    );
    insta::assert_snapshot!("catalog_summary_without_german", summary);
    Ok(())
}
