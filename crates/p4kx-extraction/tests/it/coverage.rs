//! The coverage check of a reference list over a synthetic Archive.

use std::error::Error;

use p4kx_extraction::{Archive, CoverageReport, DataForgeEntry, ReferenceKind, ReferenceList};
use p4kx_testkit::{
    ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, TestElement, TestEntry,
    TestInstance, TestRecord, TestStruct, TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

const LIST: &str = "missions record MissionItem
missions candidate MissionLocationTemplate
reputation record Faction
items component WeaponParams
items component MissileParams
items component MissingParams
vehicles entries Data/Vehicles/
";

fn weapon() -> TestValue {
    TestValue::Struct(TestInstance::new("WeaponParams"))
}

fn database() -> Vec<u8> {
    let entity = |guid: u128, path: &str| {
        TestRecord::new(
            guid,
            "EntityClassDefinition",
            format!("Entity.E{guid}"),
            path,
        )
    };
    DataForgeBuilder::new()
        .structure(TestStruct::new("Params"))
        .structure(TestStruct::new("WeaponParams").parent("Params"))
        .structure(TestStruct::new("MissileParams").parent("Params"))
        .structure(
            TestStruct::new("EntityClassDefinition")
                .array("Components", TestType::StrongPointer("Params".to_owned())),
        )
        .structure(TestStruct::new("MissionItem"))
        .structure(
            TestStruct::new("Faction")
                .field("name", TestType::String)
                .field("rival", TestType::ReferenceTo("Faction".to_owned())),
        )
        .structure(TestStruct::new("SubFaction").parent("Faction"))
        .record(
            entity(1, "libs/entities/one.xml")
                .value("Components", TestValue::Array(vec![weapon(), weapon()])),
        )
        .record(
            entity(2, "libs/entities/two.xml")
                .value("Components", TestValue::Array(vec![weapon()])),
        )
        .record(
            TestRecord::new(10, "Faction", "Faction.Good", "libs/factions/good.xml")
                .value("name", TestValue::text("good")),
        )
        .record(
            TestRecord::new(11, "SubFaction", "Faction.Sub", "libs/factions/sub.xml")
                .value("rival", TestValue::Reference(0xdead)),
        )
        .build()
}

fn vehicle(name: &str) -> Vec<u8> {
    TestElement::new("Vehicle")
        .attribute("name", name)
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn archive(dir: &std::path::Path) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut broken = vehicle("Broken");
    broken.truncate(30);
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database()))
        .entry(TestEntry::zstd("Data\\Vehicles\\Good.xml", vehicle("Good")))
        .entry(TestEntry::zstd(
            "data/vehicles/Text.xml",
            b"<Vehicle/>".to_vec(),
        ))
        .entry(TestEntry::zstd("Data\\Vehicles\\Broken.xml", broken))
        .entry(TestEntry::zstd(
            "Data\\Other\\Elsewhere.xml",
            vehicle("Else"),
        ))
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn report() -> Result<CoverageReport, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    Ok(CoverageReport::of(
        &archive,
        &forge,
        &ReferenceList::parse(LIST)?,
    )?)
}

#[test]
fn record_types_are_counted_with_the_types_deriving_from_them() -> TestResult {
    let report = report()?;

    let [item, candidate, faction] = report.types.as_slice() else {
        return Err(format!("three types: {:?}", report.types).into());
    };
    assert_eq!((item.defined, item.records), (true, 0));
    assert!(item.sample.is_none());
    assert_eq!((candidate.defined, candidate.candidate), (false, true));
    assert_eq!((faction.defined, faction.records), (true, 2));
    assert_eq!(faction.error_values, 1, "the dangling Reference");
    assert_eq!(faction.unreadable_records, 0);
    let sample = faction.sample.as_ref().ok_or("a sample")?;
    assert_eq!(sample.record_path, "libs/factions/good");
    assert!(sample.xml_bytes > 0 && sample.json_bytes > 0 && sample.error.is_none());
    Ok(())
}

#[test]
fn components_are_counted_in_the_entities_and_reached_by_a_query() -> TestResult {
    let report = report()?;

    let [weapon, missile, missing] = report.components.as_slice() else {
        return Err(format!("three components: {:?}", report.components).into());
    };
    assert_eq!((weapon.entities, weapon.instances), (2, 3));
    assert!(weapon.defined && weapon.reachable, "{weapon:?}");
    assert_eq!((missile.entities, missile.instances), (0, 0));
    assert!(missile.defined && missile.reachable);
    assert!(!missing.defined && !missing.reachable);
    Ok(())
}

#[test]
fn entries_of_a_folder_are_converted_from_cryxml() -> TestResult {
    let report = report()?;

    let [folder] = report.entries.as_slice() else {
        return Err("one folder".into());
    };
    assert_eq!(folder.entries, 3);
    assert_eq!((folder.converted, folder.text), (1, 1));
    let [(path, _reason)] = folder.failures.as_slice() else {
        return Err(format!("one failure: {:?}", folder.failures).into());
    };
    assert_eq!(path, "Data\\Vehicles\\Broken.xml");
    assert!(!report.is_complete());
    Ok(())
}

#[test]
fn the_built_in_list_is_valid_and_names_every_kind() {
    let list = ReferenceList::builtin();

    for kind in [
        ReferenceKind::Record,
        ReferenceKind::Candidate,
        ReferenceKind::Component,
        ReferenceKind::Entries,
    ] {
        assert!(
            list.items().iter().any(|item| item.kind == kind),
            "{kind:?}"
        );
    }
    assert_eq!(
        list.items()
            .iter()
            .filter(|item| item.kind == ReferenceKind::Record)
            .count(),
        37
    );
    assert_eq!(
        list.items()
            .iter()
            .filter(|item| item.kind == ReferenceKind::Component)
            .count(),
        20
    );
}

#[test]
fn a_malformed_line_is_an_error() {
    assert!(ReferenceList::parse("missions record").is_err());
    assert!(ReferenceList::parse("missions thing Name").is_err());
    assert!(ReferenceList::parse("a record B extra").is_err());
}
