//! `p4kx dcb query` with `where` conditions, end to end over a synthetic DataForge.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

fn weapon(guid: u128, name: &str, dps: f64, size: &str, ballistic: bool) -> TestRecord {
    TestRecord::new(
        guid,
        "Weapon",
        format!("Weapon.{name}"),
        format!("libs/foundry/records/weapons/{}.xml", name.to_lowercase()),
    )
    .value("name", TestValue::text(name))
    .value("dps", TestValue::Double(dps))
    .value("size", TestValue::text(size))
    .value("ballistic", TestValue::Bool(ballistic))
    .value(
        "ammo",
        TestValue::UInt32(guid.try_into().unwrap_or_default()),
    )
    .value("maker", TestValue::Guid(guid))
}

fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["S1", "S2", "S3"])
        .structure(TestStruct::new("Item").field("name", TestType::String))
        .structure(
            TestStruct::new("Weapon")
                .parent("Item")
                .field("dps", TestType::Double)
                .field("size", TestType::Enum("Size".to_owned()))
                .field("ballistic", TestType::Bool)
                .field("ammo", TestType::UInt32)
                .field("maker", TestType::Guid),
        )
        .structure(
            TestStruct::new("Broken")
                .field("x", TestType::Int32)
                .declared_size(9),
        )
        .record(weapon(1, "Arrow", 1250.5, "S1", false))
        .record(weapon(2, "Badger", 980.0, "S2", true))
        .record(weapon(3, "Cutter", f64::NAN, "S3", true))
        .record(weapon(4, "Deadbolt", 2400.0, "S3", false))
        .record(
            TestRecord::new(5, "Item", "Item.Box", "libs/foundry/records/items/box.xml")
                .value("name", TestValue::text("Box \"large\"")),
        )
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .write_to(&path)?;
    Ok(path)
}

fn query(text: &str, code: i32) -> Result<(String, String), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let output = p4kx_testkit::p4kx()?
        .args(["dcb", "query"])
        .arg(&archive)
        .arg(text)
        .output()?;
    assert_eq!(output.status.code(), Some(code), "exit code: {output:?}");
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
    ))
}

fn table(text: &str) -> Result<String, Box<dyn Error>> {
    let (stdout, stderr) = query(text, 0)?;
    assert!(stderr.is_empty(), "no warnings: {stderr}");
    Ok(stdout)
}

fn error(text: &str) -> Result<String, Box<dyn Error>> {
    let (stdout, stderr) = query(text, 2)?;
    assert!(stdout.is_empty(), "nothing is printed: {stdout}");
    Ok(stderr)
}

#[test]
fn where_keeps_the_records_whose_values_match() -> TestResult {
    insta::assert_snapshot!(table("from Weapon select name, dps where dps > 1000")?);
    Ok(())
}

#[test]
fn and_binds_tighter_than_or() -> TestResult {
    insta::assert_snapshot!(table(
        "from Weapon select name, size, ballistic \
         where size = \"S3\" or size = \"S1\" and not ballistic = false"
    )?);
    Ok(())
}

#[test]
fn nan_matches_only_not_equal() -> TestResult {
    insta::assert_snapshot!(table("from Weapon select name, dps where dps != 980")?);
    Ok(())
}

#[test]
fn strings_match_ignoring_case() -> TestResult {
    insta::assert_snapshot!(table(
        "at \"*\" select name where name contains \"DE\" or name like \"b*\"\"LARGE\"\"\""
    )?);
    Ok(())
}

#[test]
fn guids_and_integers_compare_by_value() -> TestResult {
    insta::assert_snapshot!(table(
        "from Weapon select name, maker \
         where maker > \"00000000-0000-0000-0000-000000000001\" and ammo <= 3"
    )?);
    Ok(())
}

#[test]
fn a_field_missing_from_a_record_never_matches() -> TestResult {
    insta::assert_snapshot!(table("at \"*\" select name, dps where not dps < 1000")?);
    Ok(())
}

#[test]
fn comparing_incompatible_types_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Weapon where dps = \"fast\"")?);
    Ok(())
}

#[test]
fn matching_a_number_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Weapon where ammo contains \"1\"")?);
    Ok(())
}

#[test]
fn an_invalid_guid_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Weapon where maker = \"not-a-guid\"")?);
    Ok(())
}

#[test]
fn an_unclosed_parenthesis_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Weapon where (dps > 1 or dps < 0")?);
    Ok(())
}

#[test]
fn a_missing_operator_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Weapon where dps 1000")?);
    Ok(())
}

#[test]
fn a_from_type_whose_layout_cannot_be_read_is_a_dataforge_error() -> TestResult {
    let (stdout, stderr) = query("from Broken", 5)?;

    assert!(stdout.is_empty(), "nothing is printed: {stdout}");
    insta::assert_snapshot!(stderr);
    Ok(())
}
