//! `p4kx entity loadout` end to end, over the synthetic sample ship and, with `--ignored`, over
//! the real Archive `P4KX_DATA_P4K` names.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, TestElement, TestEntity,
    TestEntry, TestLoadout, TestLoadoutEntry, TestRecord, TestStruct, TestType, entity_schema,
    real_archive_path, sample_ship,
};

type TestResult = Result<(), Box<dyn Error>>;

fn write_archive(
    dir: &Path,
    database: &DataForgeBuilder,
    extra: Vec<TestEntry>,
) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    extra
        .into_iter()
        .fold(
            ArchiveBuilder::new().entry(TestEntry::zstd("Data\\Game2.dcb", database.build())),
            ArchiveBuilder::entry,
        )
        .write_to(&path)?;
    Ok(path)
}

struct Run {
    stdout: String,
    stderr: String,
}

fn loadout(archive: &Path, entity: &str, json: bool, code: i32) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    let output = command
        .args(["entity", "loadout"])
        .arg(archive)
        .arg(entity)
        .output()?;
    assert_eq!(output.status.code(), Some(code), "exit code: {output:?}");
    let archive_text = archive.display().to_string();
    let json_text = serde_json::to_string(&archive_text)?;
    let json_text = json_text.trim_matches('"');
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?
            .replace(json_text, "<archive>")
            .replace(&archive_text, "<archive>"),
        stderr: String::from_utf8(output.stderr)?,
    })
}

fn sample(entity: &str, json: bool, code: i32) -> Result<Run, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), &sample_ship(), Vec::new())?;
    loadout(&archive, entity, json, code)
}

#[test]
fn the_loadout_tree_is_printed_as_indented_text() -> TestResult {
    let run = sample("TEST_Ship", false, 0)?;
    assert!(run.stderr.is_empty(), "no warnings: {}", run.stderr);
    insta::assert_snapshot!(run.stdout);
    Ok(())
}

#[test]
fn the_loadout_tree_is_printed_as_json_with_a_stable_shape() -> TestResult {
    let run = sample(
        "libs/foundry/records/entities/spaceships/test_ship",
        true,
        0,
    )?;
    assert_eq!(run.stdout.lines().count(), 1, "one line of JSON");
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    insta::assert_snapshot!(serde_json::to_string_pretty(&value)?);
    Ok(())
}

#[test]
fn the_same_entity_by_guid_gives_the_same_tree() -> TestResult {
    let by_class = sample("test_ship", true, 0)?.stdout;
    let by_guid = sample("0079c5d5-1678-4f8c-85ba-18ca8f642af6", true, 0)?.stdout;
    assert_eq!(by_class, by_guid, "the same Entity");
    Ok(())
}

#[test]
fn an_unknown_entity_is_invalid_input() -> TestResult {
    let run = sample("No_Such_Ship", true, 2)?;
    assert!(run.stdout.is_empty(), "no JSON: {}", run.stdout);
    assert_eq!(
        run.stderr.trim(),
        "error: no Entity has the record path, GUID or class name No_Such_Ship",
        "message"
    );
    Ok(())
}

#[test]
fn an_xml_loadout_is_read_from_the_archive() -> TestResult {
    let database = [
        TestEntity::new(0x1, "Door", "entities/door.xml")
            .loadout(TestLoadout::Xml("Scripts/Loadouts/Door.xml".to_owned())),
        TestEntity::new(0x2, "Decal", "entities/decal.xml"),
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    });
    let door = TestElement::new("Loadout").child(
        TestElement::new("Items").child(
            TestElement::new("Item")
                .attribute("portName", "hardpoint_decal")
                .attribute("itemName", "Decal"),
        ),
    );
    let file = TestEntry::stored(
        "Data\\Scripts\\Loadouts\\Door.xml",
        door.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
    );
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), &database, vec![file])?;
    let run = loadout(&archive, "Door", false, 0)?;
    insta::assert_snapshot!(run.stdout);
    Ok(())
}

#[test]
fn control_characters_in_names_are_escaped_in_the_text() -> TestResult {
    let odd = TestEntity::new(0x1, "Odd", "entities/odd.xml").loadout(TestLoadout::Manual(vec![
        TestLoadoutEntry::empty("line\nbreak\u{1b}[31m"),
    ]));
    let database = entity_schema(DataForgeBuilder::new()).record(odd.record());
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), &database, Vec::new())?;
    let run = loadout(&archive, "Odd", false, 0)?;
    assert!(
        run.stdout.contains("  line\\nbreak\u{fffd}[31m: -\n"),
        "one line, no escape sequence: {:?}",
        run.stdout
    );
    Ok(())
}

#[test]
fn an_entity_whose_record_cannot_be_read_exits_with_code_5() -> TestResult {
    let database = DataForgeBuilder::new()
        .structure(
            TestStruct::new("EntityClassDefinition")
                .declared_size(2)
                .field("Category", TestType::Int32),
        )
        .record(TestRecord::new(
            0x1,
            "EntityClassDefinition",
            "EntityClassDefinition.Broken",
            "entities/broken.xml",
        ));
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), &database, Vec::new())?;
    let run = loadout(&archive, "Broken", true, 5)?;
    assert!(run.stdout.is_empty(), "no JSON: {}", run.stdout);
    assert!(
        run.stderr
            .starts_with("error: the Record 00000000-0000-0000-0000-000000000001 cannot be read"),
        "message: {}",
        run.stderr
    );
    Ok(())
}
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn real_ships_and_a_vehicle_resolve_with_few_unresolved_ports() -> TestResult {
    let archive = real_archive_path()?;
    for entity in [
        "AEGS_Avenger_Titan",
        "ANVL_Hornet_F7C",
        "RSI_Constellation_Andromeda",
        "TMBL_Cyclone",
    ] {
        let run = loadout(&archive, entity, true, 0)?;
        let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
        let ports = value["ports"].as_u64().ok_or("ports")?;
        let attached = value["attached"].as_u64().ok_or("attached")?;
        let unresolved = value["unresolved"].as_u64().ok_or("unresolved")?;
        assert!(ports > 20, "{entity}: {ports} ports");
        assert!(attached > 10, "{entity}: {attached} attached");
        assert!(
            unresolved * 20 <= ports,
            "{entity}: {unresolved} of {ports} unresolved"
        );
    }
    Ok(())
}
