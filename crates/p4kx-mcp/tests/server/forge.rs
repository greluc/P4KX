use p4kx_testkit::{DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue};

use crate::support::{Install, TestResult};

pub(crate) const AEGS: u128 = 0x5a8d_0e3c_1f2b_4c6d_8e9f_0a1b_2c3d_4e5f;
pub(crate) const AVENGER: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
pub(crate) const GLADIUS: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0007;
pub(crate) const AEGS_TEXT: &str = "5a8d0e3c-1f2b-4c6d-8e9f-0a1b2c3d4e5f";
pub(crate) const AVENGER_TEXT: &str = "cbee34fd-6441-4000-b99f-924f7f26e19f";

/// The schema every fixture DataForge has: an enum, a Manufacturer, an Item with a typed
/// Reference to its maker, a Ship deriving from Item and a Note holding text.
fn schema() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Medium", "Large"])
        .structure(TestStruct::new("Manufacturer").field("code", TestType::String))
        .structure(
            TestStruct::new("Item")
                .field("mass", TestType::Single)
                .field("size", TestType::Enum("Size".to_owned()))
                .field("maker", TestType::ReferenceTo("Manufacturer".to_owned())),
        )
        .structure(
            TestStruct::new("Ship")
                .parent("Item")
                .field("crew", TestType::Int32)
                .array("tags", TestType::String),
        )
        .structure(TestStruct::new("Note").field("text", TestType::String))
}

fn ship(guid: u128, name: &str, crew: i32) -> TestRecord {
    TestRecord::new(
        guid,
        "Ship",
        format!("Ship.{name}"),
        format!(
            "libs/foundry/records/entities/spaceships/aegs_{}.xml",
            name.to_lowercase()
        ),
    )
    .value("crew", TestValue::Int32(crew))
    .value("mass", TestValue::Single(1000.0))
    .value("size", TestValue::text("Medium"))
    .value("maker", TestValue::Reference(AEGS))
    .value(
        "tags",
        TestValue::Array(vec![TestValue::text("fighter"), TestValue::text(name)]),
    )
}

/// A DataForge with one Manufacturer, two Ships, two Items sharing a record path, and `notes`
/// Notes of `note_len` bytes of text each, all at one record path.
pub(crate) fn dataforge(notes: usize, note_len: usize) -> Vec<u8> {
    let mut builder = schema()
        .record(
            TestRecord::new(
                AEGS,
                "Manufacturer",
                "Manufacturer.AEGS",
                "libs/foundry/records/manufacturers/aegs.xml",
            )
            .value("code", TestValue::text("AEGS")),
        )
        .record(ship(AVENGER, "Avenger", 2))
        .record(ship(GLADIUS, "Gladius", 1))
        .record(TestRecord::new(
            0x10,
            "Item",
            "Item.Box",
            "libs/foundry/records/items/boxes.xml",
        ))
        .record(TestRecord::new(
            0x11,
            "Item",
            "Item.Crate",
            "libs/foundry/records/items/boxes.xml",
        ));
    for index in 0..notes {
        builder = builder.record(
            TestRecord::new(
                0x1000 + index as u128,
                "Note",
                format!("Note.N{index:04}"),
                "libs/foundry/records/notes/long.xml",
            )
            .value("text", TestValue::text("n".repeat(note_len))),
        );
    }
    builder.build()
}

/// A DataForge with the fixture schema and `count` Items, one record path each.
pub(crate) fn many_items(count: usize) -> Vec<u8> {
    (0..count)
        .fold(schema(), |builder, index| {
            builder.record(
                TestRecord::new(
                    0x2000 + index as u128,
                    "Item",
                    format!("Item.I{index:04}"),
                    format!("libs/foundry/records/items/item_{index:04}.xml"),
                )
                .value(
                    "mass",
                    TestValue::Single(f32::from(u16::try_from(index).unwrap_or(u16::MAX))),
                ),
            )
        })
        .build()
}

/// Creates an install whose `LIVE` Archive holds `dataforge` as `Data\Game2.dcb`.
pub(crate) fn install(dataforge: Vec<u8>) -> TestResult<Install> {
    Install::new(vec![
        TestEntry::stored("readme.txt", b"hello".to_vec()),
        TestEntry::zstd("Data\\Game2.dcb", dataforge),
    ])
}
