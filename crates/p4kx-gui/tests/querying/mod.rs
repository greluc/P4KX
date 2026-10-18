//! Helpers of the Query tests: a synthetic Archive whose DataForge has typed fields.

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};
use std::error::Error;
use std::path::{Path, PathBuf};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// Items and Ships with a name, a mass and, for Ships, a crew.
pub(crate) fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Item")
                .field("name", TestType::String)
                .field("mass", TestType::Single),
        )
        .structure(
            TestStruct::new("Ship")
                .parent("Item")
                .field("crew", TestType::Int32),
        )
        .record(
            TestRecord::new(1, "Ship", "Ship.Avenger", "libs/ships/avenger.xml")
                .value("name", TestValue::text("Avenger, \"Titan\""))
                .value("mass", TestValue::Single(25_000.5))
                .value("crew", TestValue::Int32(2)),
        )
        .record(
            TestRecord::new(2, "Item", "Item.Box", "libs/items/box.xml")
                .value("name", TestValue::text("Box"))
                .value("mass", TestValue::Single(-0.25)),
        )
        .record(
            TestRecord::new(3, "Item", "Item.Crate", "libs/items/crate.xml")
                .value("name", TestValue::text("Crate"))
                .value("mass", TestValue::Single(100.0)),
        )
}

/// Items with a list of tags, a mass and a null owner.
pub(crate) fn listed() -> DataForgeBuilder {
    let tags = |values: std::ops::RangeInclusive<i32>| {
        TestValue::Array(values.map(TestValue::Int32).collect())
    };
    DataForgeBuilder::new()
        .structure(TestStruct::new("Part").field("mass", TestType::Single))
        .structure(
            TestStruct::new("Item")
                .array("tags", TestType::Int32)
                .field("mass", TestType::Single)
                .field("owner", TestType::StrongPointer("Part".to_owned())),
        )
        .record(
            TestRecord::new(1, "Item", "Item.A", "libs/items/a.xml")
                .value("tags", tags(1..=2))
                .value("mass", TestValue::Single(1.0))
                .value("owner", TestValue::Null),
        )
        .record(
            TestRecord::new(2, "Item", "Item.B", "libs/items/b.xml")
                .value("tags", TestValue::Array(Vec::new()))
                .value("mass", TestValue::Single(2.0))
                .value("owner", TestValue::Null),
        )
        .record(
            TestRecord::new(3, "Item", "Item.C", "libs/items/c.xml")
                .value("tags", tags(3..=12))
                .value("mass", TestValue::Single(3.0))
                .value("owner", TestValue::Null),
        )
}

/// `count` Items numbered from 0, with the number as mass.
pub(crate) fn numbered(count: u16) -> DataForgeBuilder {
    (0..count).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Item").field("mass", TestType::Single)),
        |builder, n| {
            builder.record(
                TestRecord::new(
                    u128::from(n) + 1,
                    "Item",
                    format!("Item.N{n}"),
                    format!("libs/items/item{n:05}.xml"),
                )
                .value("mass", TestValue::Single(f32::from(n))),
            )
        },
    )
}

/// Writes an Archive holding `forge` as `Data\Game2.dcb` into `dir`.
pub(crate) fn archive(dir: &Path, forge: &DataForgeBuilder) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge.build()))
        .write_to(&path)?;
    Ok(path)
}
