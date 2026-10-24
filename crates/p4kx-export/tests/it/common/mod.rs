// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The synthetic DataForge shared by the Record export tests.

use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

const AVENGER: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const BOX: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;
const CRATE: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;
const BROKEN: u128 = 0x0000_0009_0002_0003_0004_0000_0000_0005;

/// A ship with nested values, References, a null pointer and an error inside an array, two items sharing a record path, and a Record
/// whose struct layout cannot be read, defined out of record-path order.
#[must_use]
pub(crate) fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(TestStruct::new("Wheel").field("size", TestType::Int32))
        .structure(
            TestStruct::new("Item")
                .field("mass", TestType::Single)
                .field("size", TestType::Enum("Size".to_owned()))
                .field("label", TestType::Locale),
        )
        .structure(
            TestStruct::new("Ship")
                .parent("Item")
                .field("maker", TestType::Reference)
                .array("wheels", TestType::Class("Wheel".to_owned()))
                .field("spare", TestType::StrongPointer("Wheel".to_owned()))
                .field("front", TestType::WeakPointer("Wheel".to_owned()))
                .field("trailer", TestType::StrongPointer("Wheel".to_owned()))
                .array("parts", TestType::Reference)
                .field("name", TestType::String),
        )
        .structure(
            TestStruct::new("Odd")
                .field("a", TestType::Int8)
                .declared_size(3),
        )
        .instance(
            "spare",
            TestInstance::new("Wheel").value("size", TestValue::Int32(15)),
        )
        .record(ship())
        .record(item(BOX, "Item.Box", TestValue::Single(1.0)))
        .record(item(CRATE, "Item.Crate", TestValue::Single(f32::NAN)))
        .record(TestRecord::new(
            BROKEN,
            "Odd",
            "Odd.Broken",
            "libs/foundry/records/odd.xml",
        ))
}

fn ship() -> TestRecord {
    let wheel = |size: i32| {
        TestValue::Struct(TestInstance::new("Wheel").value("size", TestValue::Int32(size)))
    };
    TestRecord::new(
        AVENGER,
        "Ship",
        "Ship.Avenger",
        "libs/foundry/records/ships/avenger.xml",
    )
    .value("mass", TestValue::Single(25_000.5))
    .value("size", TestValue::text("Large"))
    .value("label", TestValue::text("@vehicle_NameAEGS_Avenger"))
    .value("maker", TestValue::Reference(BOX))
    .value("wheels", TestValue::Array(vec![wheel(16), wheel(17)]))
    .value("spare", TestValue::pointer("spare"))
    .value("front", TestValue::pointer("spare"))
    .value(
        "parts",
        TestValue::Array(vec![
            TestValue::Reference(BOX),
            TestValue::Null,
            TestValue::Reference(0x77),
        ]),
    )
    .value("name", TestValue::text("Avenger \"Titan\"\n"))
}

fn item(guid: u128, name: &str, mass: TestValue) -> TestRecord {
    TestRecord::new(guid, "Item", name, "libs/foundry/records/items/boxes.xml")
        .value("mass", mass)
        .value("size", TestValue::Raw(vec![0xff; 4]))
        .value("label", TestValue::text("@item_Name"))
}
