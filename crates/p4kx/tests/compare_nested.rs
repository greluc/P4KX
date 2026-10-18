//! `p4kx compare --records` end to end over Records with nested structs, arrays and References.

use std::error::Error;
use std::path::Path;

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

const OLD: &str = "LIVE/Data.p4k";
const NEW: &str = "PTU/Data.p4k";

const SHIP: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0001;
const MAKER: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0002;
const OTHER_MAKER: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0003;

struct Side {
    armor: i32,
    sizes: Vec<i32>,
    spare: bool,
    maker: Option<u128>,
    focus: &'static str,
    label: TestValue,
}

fn plate(armor: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Plate")
            .value("armor", TestValue::Int32(armor))
            .value("grade", TestValue::UInt8(1)),
    )
}

fn ship(side: Side) -> TestRecord {
    TestRecord::new(SHIP, "Ship", "Ship.Avenger", "libs/ships/avenger.xml")
        .value(
            "hull",
            TestValue::Struct(
                TestInstance::new("Hull")
                    .value("plate", plate(side.armor))
                    .value("label", side.label),
            ),
        )
        .value(
            "sizes",
            TestValue::Array(side.sizes.into_iter().map(TestValue::Int32).collect()),
        )
        .value(
            "spare",
            if side.spare {
                plate(9)
            } else {
                TestValue::Null
            },
        )
        .value(
            "maker",
            side.maker.map_or(TestValue::Null, TestValue::Reference),
        )
        .value(
            "slots",
            TestValue::Array(vec![
                TestValue::Struct(TestInstance::new("Plate").named("first")),
                TestValue::Struct(TestInstance::new("Plate").named("second")),
            ]),
        )
        .value("focus", TestValue::pointer(side.focus))
}

fn database(side: Side) -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Plate")
                .field("armor", TestType::Int32)
                .field("grade", TestType::UInt8),
        )
        .structure(
            TestStruct::new("Hull")
                .field("plate", TestType::Class("Plate".to_owned()))
                .field("label", TestType::String),
        )
        .structure(TestStruct::new("Maker").field("size", TestType::Int32))
        .structure(
            TestStruct::new("Ship")
                .field("hull", TestType::Class("Hull".to_owned()))
                .array("sizes", TestType::Int32)
                .field("spare", TestType::StrongPointer("Plate".to_owned()))
                .field("maker", TestType::Reference)
                .array("slots", TestType::Class("Plate".to_owned()))
                .field("focus", TestType::WeakPointer("Plate".to_owned())),
        )
        .record(TestRecord::new(
            MAKER,
            "Maker",
            "Maker.One",
            "libs/makers/one.xml",
        ))
        .record(TestRecord::new(
            OTHER_MAKER,
            "Maker",
            "Maker.Two",
            "libs/makers/two.xml",
        ))
        .record(ship(side))
        .build()
}

fn write(dir: &Path, archive: &str, dataforge: Vec<u8>) -> TestResult {
    let path = dir.join(archive);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Game2.dcb", dataforge))
        .write_to(path)?;
    Ok(())
}

fn report(old: Side, new: Side) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    write(dir.path(), OLD, database(old))?;
    write(dir.path(), NEW, database(new))?;
    let output = Command::cargo_bin("p4kx")?
        .current_dir(dir.path())
        .args(["compare", OLD, NEW, "--records", "--report", "report.json"])
        .output()?;
    assert_eq!(output.status.code(), Some(1), "exit code: {output:?}");
    Ok(std::fs::read_to_string(dir.path().join("report.json"))?)
}

#[test]
fn the_report_lists_nested_changes_by_location() -> TestResult {
    let old = Side {
        armor: 1,
        sizes: vec![1, 2, 3],
        spare: false,
        maker: Some(MAKER),
        focus: "first",
        label: TestValue::text("Hull"),
    };
    let new = Side {
        armor: 2,
        sizes: vec![1, 5],
        spare: true,
        maker: Some(OTHER_MAKER),
        focus: "second",
        label: TestValue::Raw(u32::MAX.to_le_bytes().to_vec()),
    };

    let report = report(old, new)?;

    insta::assert_snapshot!(report);
    Ok(())
}

#[test]
fn identical_nested_records_exit_with_code_0() -> TestResult {
    let side = || Side {
        armor: 1,
        sizes: vec![1, 2],
        spare: true,
        maker: Some(MAKER),
        focus: "second",
        label: TestValue::text("Hull"),
    };
    let dir = tempfile::tempdir()?;
    write(dir.path(), OLD, database(side()))?;
    write(dir.path(), NEW, database(side()))?;

    let output = Command::cargo_bin("p4kx")?
        .current_dir(dir.path())
        .args(["compare", OLD, NEW, "--records"])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "no difference: {output:?}");
    Ok(())
}
