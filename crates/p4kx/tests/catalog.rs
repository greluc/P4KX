//! `p4kx catalog` end to end, over a synthetic DataForge and English and German `global.ini`
//! files.

mod catalog_support;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use catalog_support::{EXPORTED_AT, english, write_archive, write_archive_with};
use p4kx_testkit::TestEntry;

type TestResult = Result<(), Box<dyn Error>>;

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

#[test]
fn catalog_with_a_corrupt_german_global_ini_cannot_read_the_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let corrupt = TestEntry::zstd(
        "Data\\Localization\\german_(germany)\\global.ini",
        b"manufacturer_NameACAS=\xff\xfe\r\n".to_vec(),
    );
    let archive = write_archive_with(dir.path(), &[english(), corrupt])?;
    let output = p4kx(&["catalog"])?
        .arg(&archive)
        .arg("--out")
        .arg(dir.path().join("corrupt.json"))
        .output()?;
    assert_eq!(output.status.code(), Some(3), "exit code: {output:?}");
    assert!(
        String::from_utf8(output.stderr)?.contains("german_(germany)"),
        "names the Entry"
    );
    Ok(())
}
