//! `p4kx dcb query` end to end, over a synthetic DataForge and, with `--ignored`, over the real
//! Archive `P4KX_DATA_P4K` names.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
    real_archive_path,
};

type TestResult = Result<(), Box<dyn Error>>;

fn schema() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(TestStruct::new("Point").field("x", TestType::Single))
        .structure(
            TestStruct::new("Item")
                .field("mass", TestType::Single)
                .field("size", TestType::Enum("Size".to_owned()))
                .field("origin", TestType::Class("Point".to_owned())),
        )
        .structure(
            TestStruct::new("Ship")
                .parent("Item")
                .field("crew", TestType::Int32)
                .field("name", TestType::String),
        )
        .structure(
            TestStruct::new("Weapon")
                .parent("Item")
                .field("damage", TestType::Double),
        )
        .structure(TestStruct::new("Tag").field("select", TestType::Bool))
}

fn database() -> DataForgeBuilder {
    schema()
        .record(
            TestRecord::new(
                0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f,
                "Ship",
                "Ship.Avenger",
                "libs/foundry/records/ships/avenger.xml",
            )
            .value("mass", TestValue::Single(25_000.5))
            .value("size", TestValue::text("Large"))
            .value("crew", TestValue::Int32(2))
            .value("name", TestValue::text("Avenger")),
        )
        .record(
            TestRecord::new(
                0x0000_0001_0002_0003_0004_0000_0000_0005,
                "Item",
                "Item.Crate",
                "libs/foundry/records/items/boxes.xml",
            )
            .value("mass", TestValue::Single(2.5))
            .value("size", TestValue::Raw(vec![0xff; 4])),
        )
        .record(
            TestRecord::new(
                0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d,
                "Item",
                "Item.Box",
                "libs/foundry/records/items/boxes.xml",
            )
            .value("mass", TestValue::Single(1.0))
            .value("size", TestValue::text("Small")),
        )
        .record(
            TestRecord::new(
                0x1111_2222_3333_4444_5555_6666_7777_8888,
                "Weapon",
                "Weapon.Gun",
                "libs/foundry/records/weapons/gun.xml",
            )
            .value("mass", TestValue::Single(12.0))
            .value("size", TestValue::text("Small"))
            .value("damage", TestValue::Double(1250.75)),
        )
        .record(
            TestRecord::new(
                0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0000,
                "Tag",
                "Tag.Hidden",
                "libs/foundry/records/tags/hidden.xml",
            )
            .value("select", TestValue::Bool(true)),
        )
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .write_to(&path)?;
    Ok(path)
}

struct Run {
    stdout: String,
    stderr: String,
}

fn query(text: &str, code: i32) -> Result<Run, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let output = p4kx_testkit::p4kx()?
        .args(["dcb", "query"])
        .arg(&archive)
        .arg(text)
        .output()?;
    assert_eq!(output.status.code(), Some(code), "exit code: {output:?}");
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

fn table(text: &str) -> Result<String, Box<dyn Error>> {
    let run = query(text, 0)?;
    assert!(run.stderr.is_empty(), "no warnings: {}", run.stderr);
    Ok(run.stdout)
}

fn error(text: &str) -> Result<String, Box<dyn Error>> {
    let run = query(text, 2)?;
    assert!(run.stdout.is_empty(), "nothing is printed: {}", run.stdout);
    Ok(run.stderr)
}

#[test]
fn from_selects_the_records_of_a_type_and_its_subtypes() -> TestResult {
    insta::assert_snapshot!(table("from Item select mass, size")?);
    Ok(())
}

#[test]
fn from_a_subtype_selects_its_own_fields() -> TestResult {
    insta::assert_snapshot!(table("FROM Ship Select crew, name, mass")?);
    Ok(())
}

#[test]
fn at_selects_records_by_record_path_glob() -> TestResult {
    insta::assert_snapshot!(table("at \"*/ITEMS/*\" select mass")?);
    Ok(())
}

#[test]
fn at_without_wildcards_matches_text_anywhere_in_the_record_path() -> TestResult {
    insta::assert_snapshot!(table("at \"records\\weapons\"")?);
    Ok(())
}

#[test]
fn from_and_at_select_together() -> TestResult {
    insta::assert_snapshot!(table("from Item at \"*/ships/*\" select mass, size")?);
    Ok(())
}

#[test]
fn without_from_a_missing_field_gives_an_empty_cell() -> TestResult {
    insta::assert_snapshot!(table("at \"*\" select crew, damage")?);
    Ok(())
}

#[test]
fn a_keyword_as_a_name_is_written_in_backticks() -> TestResult {
    insta::assert_snapshot!(table("from Tag select `select`")?);
    Ok(())
}

#[test]
fn a_query_without_matches_prints_an_empty_table() -> TestResult {
    insta::assert_snapshot!(table("from Weapon at \"*/ships/*\" select damage")?);
    Ok(())
}

#[test]
fn an_unknown_type_is_marked_with_a_suggestion() -> TestResult {
    insta::assert_snapshot!(error("from ship select crew")?);
    Ok(())
}

#[test]
fn an_unknown_field_is_marked_with_a_suggestion() -> TestResult {
    insta::assert_snapshot!(error("from Item select mass, sizes")?);
    Ok(())
}

#[test]
fn a_field_without_from_must_exist_somewhere() -> TestResult {
    insta::assert_snapshot!(error("at \"*\" select speed")?);
    Ok(())
}

#[test]
fn a_path_cannot_end_at_a_struct() -> TestResult {
    insta::assert_snapshot!(error("from Item select origin")?);
    Ok(())
}

#[test]
fn a_path_cannot_continue_behind_a_scalar() -> TestResult {
    insta::assert_snapshot!(error("from Item select mass.value")?);
    Ok(())
}

#[test]
fn a_query_without_from_or_at_is_rejected() -> TestResult {
    insta::assert_snapshot!(error("select mass")?);
    Ok(())
}

#[test]
fn an_unterminated_string_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Item at \"*/ships")?);
    Ok(())
}

#[test]
fn an_unexpected_character_is_marked_on_its_line() -> TestResult {
    insta::assert_snapshot!(error("from Item\nselect mass;")?);
    Ok(())
}

#[test]
fn clauses_out_of_order_are_rejected() -> TestResult {
    insta::assert_snapshot!(error("select mass from Item")?);
    Ok(())
}

#[test]
fn a_missing_path_is_marked_at_the_end() -> TestResult {
    insta::assert_snapshot!(error("from Item select mass,")?);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_documented_examples_select_records_of_a_real_archive() -> TestResult {
    let archive = real_archive_path()?;
    let examples = [
        ("from EntityClassDefinition", None),
        ("at \"libs/foundry/records/entities/spaceships/*\"", None),
        (
            "from EntityClassDefinition at \"*/spaceships/*\" select Category, Invisible",
            Some("false"),
        ),
        ("at \"*/spaceships/*\" select Icon", Some(".")),
        (
            "from AmmoParams at \"*/vehicle/*\" select speed, lifetime \
             where speed >= 1500 and lifetime <= 3",
            Some("1500"),
        ),
        (
            "from EntityClassDefinition at \"*/spaceships/*\" select Category, Invisible \
             where Category = \"Default\" and not Invisible = true",
            Some("Default"),
        ),
        (
            "at \"*/spaceships/*\" select Icon, Category \
             where Icon like \"default.*\" and (Category contains \"def\" or Invisible = true)",
            Some("Default.bmp"),
        ),
    ];
    for (text, value) in examples {
        let output = p4kx_testkit::p4kx()?
            .args(["dcb", "query"])
            .arg(&archive)
            .arg(text)
            .output()?;
        assert_eq!(output.status.code(), Some(0), "{text}: {output:?}");
        let stdout = String::from_utf8(output.stdout)?;
        let rows: Vec<&str> = stdout.lines().skip(2).collect();
        assert!(rows.len() > 1, "{text} selects Records: {stdout}");
        if let Some(value) = value {
            assert!(
                rows.iter()
                    .any(|row| row.split("  ").skip(1).any(|cell| cell.contains(value))),
                "{text} reads values: {stdout}"
            );
        }
    }
    Ok(())
}
