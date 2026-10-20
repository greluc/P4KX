//! `p4kx dcb query` with paths into nested structs, arrays and polymorphic arrays, over a
//! synthetic DataForge and, with `--ignored`, over the real Archive `P4KX_DATA_P4K` names.

use std::error::Error;

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue, real_archive_path,
};

type TestResult = Result<(), Box<dyn Error>>;

fn vector(x: f32, y: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Vector")
            .value("x", TestValue::Single(x))
            .value("y", TestValue::Single(y)),
    )
}

fn component(type_name: &str, name: &str, field: &str, value: TestValue) -> TestValue {
    TestValue::Struct(
        TestInstance::new(type_name)
            .value("name", TestValue::text(name))
            .value(field, value),
    )
}

fn schema() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Vector")
                .field("x", TestType::Single)
                .field("y", TestType::Single),
        )
        .structure(TestStruct::new("Component").field("name", TestType::String))
        .structure(
            TestStruct::new("Weapon")
                .parent("Component")
                .field("damage", TestType::Double),
        )
        .structure(
            TestStruct::new("Shield")
                .parent("Component")
                .field("hp", TestType::Single),
        )
        .structure(
            TestStruct::new("Ship")
                .field("position", TestType::Class("Vector".to_owned()))
                .field("anchor", TestType::StrongPointer("Vector".to_owned()))
                .array(
                    "Components",
                    TestType::StrongPointer("Component".to_owned()),
                )
                .array("tags", TestType::String)
                .array("hardpoints", TestType::Class("Vector".to_owned())),
        )
}

fn database() -> Vec<u8> {
    schema()
        .record(
            TestRecord::new(1, "Ship", "Ship.Avenger", "libs/ships/avenger.xml")
                .value("position", vector(1.5, -2.0))
                .value("anchor", vector(7.0, 8.0))
                .value(
                    "Components",
                    TestValue::Array(vec![
                        component("Weapon", "Laser", "damage", TestValue::Double(250.5)),
                        component("Shield", "Bubble", "hp", TestValue::Single(900.0)),
                        component("Weapon", "Cannon", "damage", TestValue::Double(1200.0)),
                    ]),
                )
                .value(
                    "tags",
                    TestValue::Array(vec![TestValue::text("fast"), TestValue::text("small")]),
                )
                .value(
                    "hardpoints",
                    TestValue::Array(vec![vector(1.0, 2.0), vector(3.0, 4.0)]),
                ),
        )
        .record(
            TestRecord::new(2, "Ship", "Ship.Aurora", "libs/ships/aurora.xml")
                .value(
                    "Components",
                    TestValue::Array(vec![
                        TestValue::Null,
                        component("Shield", "Wall", "hp", TestValue::Single(10.0)),
                    ]),
                )
                .value("tags", TestValue::Array(vec![TestValue::text("slow")])),
        )
        .structure(TestStruct::new("Wreck").array("tags", TestType::String))
        .record(
            TestRecord::new(3, "Wreck", "Wreck.Hull", "libs/wrecks/hull.xml").value(
                "tags",
                TestValue::Array(
                    (1..=12)
                        .map(|tag| TestValue::text(format!("t{tag}")))
                        .collect(),
                ),
            ),
        )
        .build()
}

fn query(text: &str, code: i32) -> Result<(String, String), Box<dyn Error>> {
    query_as(text, "table", code)
}

fn query_as(text: &str, format: &str, code: i32) -> Result<(String, String), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database()).encrypted())
        .write_to(&archive)?;
    let output = p4kx_testkit::p4kx()?
        .args(["dcb", "query", "--format", format])
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
fn lists_are_json_arrays_in_csv_and_in_json() -> TestResult {
    let text = "from Ship select tags[*], Components[*].name, Components[Shield].hp, tags[0]";
    let (csv, _) = query_as(text, "csv", 0)?;
    let (json, _) = query_as(text, "json", 0)?;

    insta::assert_snapshot!(format!("{csv}\n{json}"));
    Ok(())
}

#[test]
fn a_long_list_is_shortened_in_the_table_only() -> TestResult {
    let text = "from Wreck select tags[*]";
    let (csv, _) = query_as(text, "csv", 0)?;

    insta::assert_snapshot!(format!("{}\n{csv}", table(text)?));
    Ok(())
}

#[test]
fn dot_paths_reach_into_inline_structs_and_strong_pointers() -> TestResult {
    insta::assert_snapshot!(table("from Ship select position.x, position.y, anchor.y")?);
    Ok(())
}

#[test]
fn an_index_selects_one_element_or_null() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select tags[0], tags[1], Components[0].name, hardpoints[1].y"
    )?);
    Ok(())
}

#[test]
fn a_star_lists_every_element_in_one_cell() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select tags[*], Components[*].name, hardpoints[*].x"
    )?);
    Ok(())
}

#[test]
fn a_component_selector_picks_the_elements_of_a_type() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select Components[Weapon].name, Components[Weapon].damage, \
         Components[Shield].hp"
    )?);
    Ok(())
}

#[test]
fn where_holds_when_one_of_several_values_holds() -> TestResult {
    insta::assert_snapshot!(table(
        "from Ship select Components[Weapon].damage where Components[Weapon].damage > 1000 \
         or tags[*] = \"slow\""
    )?);
    Ok(())
}

#[test]
fn an_unknown_type_in_brackets_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Ship select Components[Weapn].damage")?);
    Ok(())
}

#[test]
fn an_unknown_nested_field_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Ship select Components[Shield].damage")?);
    Ok(())
}

#[test]
fn an_array_needs_its_elements_selected() -> TestResult {
    insta::assert_snapshot!(error("from Ship select hardpoints.x")?);
    Ok(())
}

#[test]
fn an_invalid_index_is_marked() -> TestResult {
    insta::assert_snapshot!(error("from Ship select tags[-1]")?);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_documented_component_examples_select_values_of_a_real_archive() -> TestResult {
    let archive = real_archive_path()?;
    let examples = [
        (
            "from EntityClassDefinition at \"libs/foundry/records/entities/spaceships/*\" \
             select Components[VehicleComponentParams].vehicleName, \
             Components[VehicleComponentParams].crewSize \
             where Components[VehicleComponentParams].crewSize >= 4",
            "[@vehicle_Name",
        ),
        (
            "from EntityClassDefinition at \"*/ships/weapons/*\" \
             select Components[SAttachableComponentParams].AttachDef.Size, \
             Components[SAttachableComponentParams].AttachDef.Type \
             where Components[SAttachableComponentParams].AttachDef.Size >= 7",
            "[WeaponGun]",
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
        assert!(
            rows.iter().any(|row| row.contains(value)),
            "{text} reads values: {stdout}"
        );
    }
    Ok(())
}
