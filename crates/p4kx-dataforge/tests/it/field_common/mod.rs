// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The synthetic DataForge of the field statistics tests and helpers to read their result.

use std::error::Error;

use p4kx_dataforge::{FieldStat, FieldStatistics};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) fn schema() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Vector")
                .field("x", TestType::Single)
                .field("y", TestType::Single),
        )
        .structure(TestStruct::new("Params"))
        .structure(
            TestStruct::new("WeaponParams")
                .parent("Params")
                .field("fireRate", TestType::Single),
        )
        .structure(
            TestStruct::new("Ship")
                .field("name", TestType::String)
                .field("mass", TestType::Double)
                .field("crew", TestType::Int32)
                .field("position", TestType::Class("Vector".to_owned()))
                .field("anchor", TestType::StrongPointer("Params".to_owned()))
                .field("link", TestType::Reference)
                .array("seats", TestType::Int32)
                .array("Components", TestType::StrongPointer("Params".to_owned())),
        )
        .structure(TestStruct::new("Crate").field("name", TestType::String))
}

pub(crate) fn weapon(rate: f32) -> TestValue {
    TestValue::Struct(TestInstance::new("WeaponParams").value("fireRate", TestValue::Single(rate)))
}

pub(crate) fn plain() -> TestValue {
    TestValue::Struct(TestInstance::new("Params"))
}

pub(crate) fn position(y: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Vector")
            .value("x", TestValue::Single(1.0))
            .value("y", TestValue::Single(y)),
    )
}

pub(crate) fn ship(index: u128, path: &str) -> TestRecord {
    TestRecord::new(index, "Ship", format!("Ship.S{index}"), path)
}

pub(crate) fn database() -> Vec<u8> {
    schema()
        .record(
            ship(1, "ships/one.xml")
                .value("name", TestValue::text("a"))
                .value("mass", TestValue::Double(1.5))
                .value("crew", TestValue::Int32(0))
                .value("position", position(-0.0))
                .value(
                    "seats",
                    TestValue::Array(vec![TestValue::Int32(1), TestValue::Int32(3)]),
                )
                .value(
                    "Components",
                    TestValue::Array(vec![weapon(2.0), weapon(6.0), plain()]),
                ),
        )
        .record(
            ship(2, "ships/two.xml")
                .value("name", TestValue::text(""))
                .value("mass", TestValue::Double(-0.0))
                .value("crew", TestValue::Int32(4))
                .value("position", position(0.0))
                .value("seats", TestValue::Array(Vec::new()))
                .value("Components", TestValue::Array(vec![weapon(-1.5)])),
        )
        .record(
            ship(3, "ships/three.xml")
                .value("name", TestValue::text("a"))
                .value("mass", TestValue::Double(3.0))
                .value("crew", TestValue::Int32(-2))
                .value("position", position(0.0))
                .value("seats", TestValue::Array(vec![TestValue::Int32(3)])),
        )
        .record(
            TestRecord::new(4, "Crate", "Crate.One", "crates/one.xml")
                .value("name", TestValue::text("box")),
        )
        .build()
}

pub(crate) fn stat<'s>(
    statistics: &'s FieldStatistics,
    path: &str,
) -> Result<&'s FieldStat, String> {
    statistics
        .get(path)
        .ok_or_else(|| format!("no statistics for {path}: {:?}", paths(statistics)))
}

pub(crate) fn paths(statistics: &FieldStatistics) -> Vec<&str> {
    statistics.fields().iter().map(FieldStat::path).collect()
}
