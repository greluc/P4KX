use std::error::Error;
use std::path::PathBuf;

use p4kx_extraction::Archive;
use p4kx_testkit::{
    ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, DataForgeBuilder, GlobalIniBuilder,
    TestElement, TestEntry, TestInstance, TestRecord, TestStruct, TestType, TestValue, test_key,
};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const NEVER: &(dyn Fn() -> bool + Sync) = &|| false;

pub(crate) fn forge_bytes() -> Vec<u8> {
    DataForgeBuilder::new()
        .enumeration("Role", ["Combat", "Hauler"])
        .structure(TestStruct::new("Part").field("note", TestType::String))
        .structure(
            TestStruct::new("Gear")
                .field("name", TestType::String)
                .field("label", TestType::Locale)
                .field("role", TestType::Enum("Role".to_owned()))
                .field("count", TestType::Int32)
                .array("tags", TestType::String)
                .array("parts", TestType::Class("Part".to_owned())),
        )
        .record(
            TestRecord::new(
                0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f,
                "Gear",
                "Gear.AEGS_Gladius",
                "libs/ships/aegs_gladius.xml",
            )
            .value("name", TestValue::text("AEGS_Gladius Fighter"))
            .value("label", TestValue::text("@vehicle_NameAEGS_Gladius"))
            .value("role", TestValue::text("Combat"))
            .value("count", TestValue::Int32(7))
            .value(
                "tags",
                TestValue::Array(vec![
                    TestValue::text("alpha"),
                    TestValue::text("BETA gladius"),
                    TestValue::text("gamma gladius"),
                ]),
            )
            .value(
                "parts",
                TestValue::Array(vec![part("plain"), part("left GLADIATOR\u{1b}[31m")]),
            ),
        )
        .record(
            TestRecord::new(
                0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d,
                "Gear",
                "Gear.Avenger",
                "libs/ships/aegs_avenger.xml",
            )
            .value("name", TestValue::text("Avenger Titan"))
            .value("label", TestValue::text("@vehicle_NameAEGS_Avenger"))
            .value("role", TestValue::text("Hauler")),
        )
        .record(
            TestRecord::new(
                0x1111_2222_3333_4444_5555_6666_7777_8888,
                "Gear",
                "Gear.Strasse",
                "libs/other/strasse.xml",
            )
            .value("name", TestValue::text("Stra\u{df}e IN GL\u{c4}DIUS"))
            .value("label", TestValue::text("@none"))
            .value("role", TestValue::text("Hauler")),
        )
        .build()
}

fn part(note: &str) -> TestValue {
    TestValue::Struct(TestInstance::new("Part").value("note", TestValue::text(note)))
}

pub(crate) fn english() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("vehicle_NameAEGS_Gladius", "Aegis Gladius")
        .text("item_Gun,P", "A gladius gun")
        .text("item_Other", "Nothing here")
        .build()
}

pub(crate) fn german() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("vehicle_NameAEGS_Gladius", "Aegis Gladius J\u{e4}ger")
        .build()
}

pub(crate) fn ship_xml() -> Vec<u8> {
    b"<Ship>\n  <Part name=\"AEGS_Gladius\"/>\n  <Part name=\"other\"/>\r\n</Ship>".to_vec()
}

pub(crate) fn cry_xml() -> Vec<u8> {
    TestElement::new("Material")
        .attribute("Name", "gladius_hull")
        .child(TestElement::new("Texture").attribute("File", "textures/other.dds"))
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

pub(crate) fn archive_builder() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge_bytes()))
        .entry(TestEntry::stored(
            "Data\\Localization\\english\\global.ini",
            english(),
        ))
        .entry(TestEntry::stored(
            "Data\\Localization\\german_(germany)\\global.ini",
            german(),
        ))
        .entry(TestEntry::deflated("Data\\Scripts\\ship.xml", ship_xml()))
        .entry(TestEntry::deflated("Data\\Libs\\hull.mtl", cry_xml()))
        .entry(TestEntry::stored(
            "Data\\Scripts\\binary.xml",
            b"gladius\0\x01\x02".to_vec(),
        ))
        .entry(TestEntry::stored("Data\\Scripts\\secret.xml", b"<gladius/>".to_vec()).encrypted())
        .entry(TestEntry::stored(
            "Data\\Scripts\\texture.dds",
            b"gladius".to_vec(),
        ))
}

pub(crate) struct Fixture {
    pub(crate) archive: Archive,
    _dir: tempfile::TempDir,
}

pub(crate) fn fixture(builder: &ArchiveBuilder) -> Result<Fixture, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path: PathBuf = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    Ok(Fixture {
        archive: Archive::open(&path)?,
        _dir: dir,
    })
}

pub(crate) fn keyed(fixture: &Fixture) {
    fixture.archive.set_key(test_key());
}
