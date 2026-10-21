//! `p4kx entity loadout` with a vehicle implementation and its Modification in the Archive.

use std::error::Error;
use std::path::PathBuf;

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntity, TestEntry, TestPort, entity_schema,
    landing_system_record,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const DIRECTORY: &str = "Data\\Scripts\\Entities\\Vehicles\\Implementations\\Xml";

const BASE: &str = r#"<Vehicle name="TEST_Ship"><Parts><Part name="body" class="Animated"><Parts>
  <Part name="hardpoint_gun" class="ItemPort"><ItemPort id="portGun" flags="invisible"/></Part>
  <Part name="hardpoint_seat" class="ItemPort"><ItemPort flags=""/></Part>
  <Part name="left_tread" class="Tread"><Tread><Wheels><Wheel partName="wheel_1"/></Wheels></Tread></Part>
  <Part name="wheel_1" class="SubPartWheel"/>
</Parts></Part></Parts>
<Modifications><Modification name="Open"><Elems>
  <Elem idRef="portGun" name="flags" value=""/>
</Elems></Modification></Modifications></Vehicle>"#;

fn archive(dir: &std::path::Path, modification: &str) -> Result<PathBuf, Box<dyn Error>> {
    let ship = TestEntity::new(0x1, "TEST_Ship", "entities/test_ship.xml")
        .vehicle(
            "Scripts/Entities/Vehicles/Implementations/Xml/TEST_Ship.xml",
            modification,
        )
        .port(TestPort::new("hardpoint_gun"))
        .port(TestPort::new("hardpoint_seat"))
        .landing_system(0x10);
    let gear = landing_system_record(
        0x10,
        "TEST_Ship_LandingSystem",
        &[("landing_gear_front", "Objects/gear.cgf", 10.0)],
    );
    let database = entity_schema(DataForgeBuilder::new())
        .record(ship.record())
        .record(gear);
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()))
        .entry(TestEntry::zstd(
            format!("{DIRECTORY}\\TEST_Ship.xml"),
            BASE.as_bytes().to_vec(),
        ))
        .write_to(&path)?;
    Ok(path)
}

fn run(archive: &std::path::Path, json: bool) -> Result<String, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    let output = command
        .args(["entity", "loadout"])
        .arg(archive)
        .arg("TEST_Ship")
        .output()?;
    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn the_text_shows_the_vehicle_treads_wheels_landing_gear_and_invisible_ports() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = run(&archive(dir.path(), "")?, false)?;
    assert!(
        text.contains("  vehicle: Scripts/Entities/Vehicles/Implementations/Xml/TEST_Ship.xml\n"),
        "{text}"
    );
    assert!(
        text.contains("  tread: left_tread (wheels wheel_1)\n"),
        "{text}"
    );
    assert!(text.contains("  wheels: wheel_1\n"), "{text}");
    assert!(
        text.contains("  landing gear: landing_gear_front\n"),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("  hardpoint_gun:") && line.ends_with("(invisible)")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.starts_with("  hardpoint_seat:") && !line.contains("invisible")),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_modification_changes_the_visibility_in_the_json() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path(), "Open")?;
    let json: Value = serde_json::from_str(&run(&archive, true)?)?;
    let entity = json.get("entity").ok_or("no entity")?;
    let vehicle = entity.get("vehicle").ok_or("no vehicle")?;
    assert_eq!(vehicle["modification"], "Open");
    assert_eq!(vehicle["treads"][0]["wheels"][0], "wheel_1");
    assert_eq!(vehicle["wheels"][0], "wheel_1");
    assert_eq!(entity["landing_gear"][0]["bone"], "landing_gear_front");
    let visibility: Vec<_> = entity["ports"]
        .as_array()
        .ok_or("no ports")?
        .iter()
        .map(|port| port["visibility"].as_str())
        .collect();
    assert_eq!(
        visibility,
        [Some("visible"), Some("visible")],
        "the Modification clears invisible"
    );
    let plain: Value = serde_json::from_str(&run(&self::archive(dir.path(), "")?, true)?)?;
    assert_eq!(plain["entity"]["ports"][0]["visibility"], "invisible");
    Ok(())
}
