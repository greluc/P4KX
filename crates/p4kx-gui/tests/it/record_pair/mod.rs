// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A pair of Archives whose synthetic DataForges differ in every kind of Record change, and a
//! Comparison of them on the Record level.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_gui::ComparisonLevels;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue,
};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

const MAKER: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0001;
const OTHER_MAKER: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0002;
const ITEM: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0003;
const SHIP: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0004;
const GONE: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0005;
const FRESH: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0006;
const WANDERER: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0007;
const BROKEN: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0008;
const RENAMED: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0009;
const RETYPED: u128 = 0x7000_0000_0000_4000_8000_0000_0000_000a;

fn hull(label: TestValue, armor: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Hull")
            .value("label", label)
            .value("armor", TestValue::Int32(armor)),
    )
}

fn drift(new: bool) -> DataForgeBuilder {
    if new {
        DataForgeBuilder::new()
            .enumeration("Rating", ["Low", "High"])
            .structure(
                TestStruct::new("Probe")
                    .field("level", TestType::Single)
                    .field("tag", TestType::String),
            )
    } else {
        DataForgeBuilder::new()
            .structure(TestStruct::new("Probe").field("level", TestType::Int32))
            .structure(TestStruct::new("Legacy").field("flag", TestType::Bool))
    }
}

fn schema(new: bool) -> DataForgeBuilder {
    drift(new)
        .structure(TestStruct::new("Maker").field("rank", TestType::Int32))
        .structure(
            TestStruct::new("Hull")
                .field("label", TestType::String)
                .field("armor", TestType::Int32),
        )
        .structure(TestStruct::new("Node").field("weight", TestType::Int32))
        .structure(
            TestStruct::new("Item")
                .field("maker", TestType::Reference)
                .array("nodes", TestType::Class("Node".to_owned()))
                .field("focus", TestType::WeakPointer("Node".to_owned()))
                .field("hull", TestType::Class("Hull".to_owned()))
                .field("speed", TestType::Int32),
        )
        .structure(TestStruct::new("Ship").field("speed", TestType::Int32))
        .structure(
            TestStruct::new("Broken")
                .field("armor", TestType::Int32)
                .declared_size(if new { 8 } else { 4 }),
        )
}

fn item(new: bool) -> TestRecord {
    let node = |name: &str| TestValue::Struct(TestInstance::new("Node").named(name));
    TestRecord::new(ITEM, "Item", "Item.Sample", "libs/items/sample.xml")
        .value(
            "maker",
            TestValue::Reference(if new { OTHER_MAKER } else { MAKER }),
        )
        .value(
            "nodes",
            TestValue::Array(vec![node("first"), node("second")]),
        )
        .value(
            "focus",
            TestValue::pointer(if new { "second" } else { "first" }),
        )
        .value(
            "hull",
            if new {
                hull(TestValue::Raw(u32::MAX.to_le_bytes().to_vec()), 2)
            } else {
                hull(TestValue::text("Hull"), 1)
            },
        )
        .value("speed", TestValue::Int32(1))
}

fn database(new: bool) -> DataForgeBuilder {
    let pick = |old: i32, new_value: i32| if new { new_value } else { old };
    let builder = schema(new)
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
        .record(item(new))
        .record(
            TestRecord::new(SHIP, "Ship", "Ship.Tuned", "libs/ships/tuned.xml")
                .value("speed", TestValue::Int32(pick(4, 40))),
        )
        .record(
            TestRecord::new(
                WANDERER,
                "Ship",
                "Ship.Wanderer",
                if new {
                    "libs/ships/moved/wanderer.xml"
                } else {
                    "libs/ships/wanderer.xml"
                },
            )
            .value("speed", TestValue::Int32(3)),
        )
        .record(TestRecord::new(
            BROKEN,
            "Broken",
            "Broken.One",
            "libs/broken/one.xml",
        ));
    builder
        .record(only_one_side(new))
        .record(renamed(new))
        .record(retyped(new))
}

fn renamed(new: bool) -> TestRecord {
    let name = if new { "Ship.After" } else { "Ship.Before" };
    TestRecord::new(RENAMED, "Ship", name, "libs/ships/renamed.xml")
        .value("speed", TestValue::Int32(5))
}

fn retyped(new: bool) -> TestRecord {
    if new {
        TestRecord::new(RETYPED, "Maker", "Retyped.One", "libs/ships/retyped.xml")
            .value("rank", TestValue::Int32(3))
    } else {
        TestRecord::new(RETYPED, "Ship", "Retyped.One", "libs/ships/retyped.xml")
            .value("speed", TestValue::Int32(2))
    }
}

fn only_one_side(new: bool) -> TestRecord {
    if new {
        TestRecord::new(FRESH, "Ship", "Ship.Fresh", "libs/ships/fresh.xml")
            .value("speed", TestValue::Int32(9))
    } else {
        TestRecord::new(GONE, "Maker", "Maker.Gone", "libs/makers/gone.xml")
            .value("rank", TestValue::Int32(2))
    }
}

fn write(dir: &Path, channel: &str, new: bool) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Game2.dcb", database(new).build()))
        .write_to(&path)?;
    Ok(path)
}

/// Writes the old Archive to `LIVE` and the new one to `PTU` in a new temporary folder.
pub(crate) fn pair() -> Result<(TempDir, PathBuf, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "LIVE", false)?;
    let new = write(dir.path(), "PTU", true)?;
    Ok((dir, old, new))
}

/// Both levels.
pub(crate) const BOTH: ComparisonLevels = ComparisonLevels {
    entries: true,
    records: true,
};
