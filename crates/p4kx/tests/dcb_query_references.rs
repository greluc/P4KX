//! `p4kx dcb query` following References with `->`, chained and behind array elements, over a
//! synthetic DataForge and, with `--ignored`, over the real Archive `P4KX_DATA_P4K` names.

use std::error::Error;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
    real_archive_path,
};

type TestResult = Result<(), Box<dyn Error>>;

const AEGIS: u128 = 10;
const TERRA: u128 = 20;
const MISSING: u128 = 99;

fn reference_to(name: &str) -> TestType {
    TestType::ReferenceTo(name.to_owned())
}

fn ship(guid: u128, name: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "Ship",
        format!("Ship.{name}"),
        format!("libs/ships/{}.xml", name.to_ascii_lowercase()),
    )
    .value("name", TestValue::text(name))
}

fn database() -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Planet")
                .field("name", TestType::String)
                .field("mass", TestType::Single),
        )
        .structure(
            TestStruct::new("Manufacturer")
                .field("name", TestType::String)
                .field("home", reference_to("Planet")),
        )
        .structure(
            TestStruct::new("Ship")
                .field("name", TestType::String)
                .field("maker", reference_to("Manufacturer"))
                .array("escorts", reference_to("Ship"))
                .field("mass", TestType::Single),
        )
        .record(
            TestRecord::new(TERRA, "Planet", "Planet.Terra", "libs/planets/terra.xml")
                .value("name", TestValue::text("Terra"))
                .value("mass", TestValue::Single(5.5)),
        )
        .record(
            TestRecord::new(
                AEGIS,
                "Manufacturer",
                "Manufacturer.Aegis",
                "libs/makers/aegis.xml",
            )
            .value("name", TestValue::text("Aegis"))
            .value("home", TestValue::Reference(TERRA)),
        )
        .record(
            ship(1, "Alpha")
                .value("maker", TestValue::Reference(AEGIS))
                .value(
                    "escorts",
                    TestValue::Array(vec![
                        TestValue::Reference(2),
                        TestValue::Reference(MISSING),
                        TestValue::Null,
                    ]),
                )
                .value("mass", TestValue::Single(100.0)),
        )
        .record(
            ship(2, "Beta")
                .value("escorts", TestValue::Array(vec![TestValue::Reference(1)]))
                .value("mass", TestValue::Single(50.0)),
        )
        .record(ship(3, "Gamma").value("maker", TestValue::Reference(MISSING)))
        .build()
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let archive = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database()).encrypted())
        .write_to(&archive)?;
    Ok(archive)
}

fn query(args: &[&str], text: &str, code: i32) -> Result<(String, String), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let output = Command::cargo_bin("p4kx")?
        .args(["dcb", "query"])
        .args(args)
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
    let (stdout, stderr) = query(&[], text, 0)?;
    assert!(stderr.is_empty(), "no warnings: {stderr}");
    Ok(stdout)
}

fn error(text: &str) -> Result<String, Box<dyn Error>> {
    let (stdout, stderr) = query(&[], text, 2)?;
    assert!(stdout.is_empty(), "nothing is printed: {stdout}");
    Ok(stderr)
}

#[test]
fn chained_references_reach_values_of_the_referenced_records() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select name, maker->name, maker->home->name, maker->home->mass"
    )?);
    Ok(())
}

#[test]
fn an_arrow_follows_every_element_of_an_array_of_references() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select name, escorts[*]->name, escorts[0]->maker->name, \
         escorts[*]->escorts[*]->name"
    )?);
    Ok(())
}

#[test]
fn an_arrow_filters_in_where() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select name where maker->home->name = \"Terra\" \
         or escorts[*]->maker->name = \"Aegis\""
    )?);
    Ok(())
}

#[test]
fn references_are_written_as_values_in_csv_and_json() -> TestResult {
    let text = "from Ship select maker->name, escorts[*]->mass";
    let (csv, _) = query(&["--format", "csv"], text, 0)?;
    let (json, _) = query(&["--format", "json"], text, 0)?;

    insta::assert_snapshot!(format!("{csv}\n{json}"));
    Ok(())
}

#[test]
fn the_summary_counts_unresolved_references() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let file = dir.path().join("result.csv");
    let file_text = file.display().to_string();
    let run = |json: bool| -> Result<std::process::Output, Box<dyn Error>> {
        let mut command = Command::cargo_bin("p4kx")?;
        command.args(["dcb", "query", "--format", "csv", "--out", &file_text]);
        if json {
            command.arg("--json");
        }
        Ok(command
            .arg(&archive)
            .arg("from Ship select maker->home->name")
            .output()?)
    };

    let text = run(false)?;
    assert_eq!(text.status.code(), Some(0), "exit code: {text:?}");
    assert_eq!(
        String::from_utf8(text.stdout)?,
        format!("3 Records written to {file_text}, 2 unresolved References\n")
    );
    let json = run(true)?;
    let summary: serde_json::Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(summary["unresolved_references"], 2, "{summary}");
    Ok(())
}

#[test]
fn an_arrow_on_a_value_that_is_no_reference_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Ship select mass->name")?);
    Ok(())
}

#[test]
fn an_arrow_on_an_array_of_references_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Ship select escorts->name")?);
    Ok(())
}

#[test]
fn a_dot_behind_a_reference_points_to_the_arrow() -> TestResult {
    insta::assert_snapshot!(error("from Ship where maker.name = \"Aegis\"")?);
    Ok(())
}

#[test]
fn an_unknown_field_behind_an_arrow_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Ship select maker->home->nam")?);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_documented_reference_examples_select_values_of_a_real_archive() -> TestResult {
    let archive = real_archive_path()?;
    let examples = [
        (
            "from EntityClassDefinition at \"*/ships/weapons/*\" \
             select Components[SAttachableComponentParams].AttachDef.Manufacturer->Code, \
             Components[SAttachableComponentParams].AttachDef.Manufacturer->Localization.Name \
             where Components[SAttachableComponentParams].AttachDef.Manufacturer->Code = \"BEH\"",
            "[@manufacturer_Name",
        ),
        (
            "from EntityClassDefinition at \"libs/foundry/records/entities/spaceships/*\" \
             select Components[SItemPortContainerComponentParams].Ports[*].defaultItem\
             .entityClass->Components[SAttachableComponentParams].AttachDef.Type",
            "unresolved References",
        ),
    ];
    for (text, value) in examples {
        let output = Command::cargo_bin("p4kx")?
            .args(["dcb", "query"])
            .arg(&archive)
            .arg(text)
            .output()?;
        assert_eq!(output.status.code(), Some(0), "{text}: {output:?}");
        let stdout = String::from_utf8(output.stdout)?;
        let rows: Vec<&str> = stdout.lines().skip(2).collect();
        assert!(rows.len() > 1, "{text} selects Records: {stdout}");
        assert!(
            rows.iter().any(|row| row.contains(value)),
            "{text} reads values: {stdout}"
        );
        assert!(
            !stdout.contains("error: "),
            "{text} reads every value: {stdout}"
        );
    }
    Ok(())
}
