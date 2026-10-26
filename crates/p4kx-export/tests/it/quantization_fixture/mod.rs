// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic Archive with quality quantizations and the `ResourceType`s that link to them.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestStruct, TestType, TestValue,
};

pub(crate) const GOLD: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const IRON: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const ORPHAN: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const LEFTOVER: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0004;
pub(crate) const NO_BANDS: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0005;
pub(crate) const REVERSED: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0006;
pub(crate) const ANOTHER: u128 = 0xa100_0000_0000_4000_8000_0000_0000_0007;
pub(crate) const ORE_GOLD: u128 = 0xb200_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const SHARED_GOLD: u128 = 0xb200_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const TWO_PROPERTIES: u128 = 0xb200_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const ORE_IRON: u128 = 0xb200_0000_0000_4000_8000_0000_0000_0004;
const PLAIN: u128 = 0xb200_0000_0000_4000_8000_0000_0000_0005;

const QUANTIZATIONS: &str = "libs/foundry/records/crafting/qualityquantization/";
const RESOURCES: &str = "libs/foundry/records/resourcetypedatabase/resourcetypedatabase";

fn structures(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder
        .structure(
            TestStruct::new("CraftingQualityQuantizationBand")
                .field("start", TestType::Int32)
                .field("end", TestType::Int32)
                .field("mappedValue", TestType::Int32),
        )
        .structure(TestStruct::new("CraftingQualityQuantization").array(
            "bands",
            TestType::Class("CraftingQualityQuantizationBand".into()),
        ))
        .structure(TestStruct::new("CraftingQualityQuantizationRecord").field(
            "qualityQuantization",
            TestType::Class("CraftingQualityQuantization".into()),
        ))
        .structure(
            TestStruct::new("CraftingQualityQuantization_RecordRef")
                .field("qualityQuantizationRecord", TestType::Reference),
        )
        .structure(
            TestStruct::new("ResourceTypeCraftingData")
                .field("name", TestType::String)
                .field(
                    "qualityQuantization",
                    TestType::Class("CraftingQualityQuantization_RecordRef".into()),
                ),
        )
        .structure(TestStruct::new("ResourceType").array(
            "properties",
            TestType::Class("ResourceTypeCraftingData".into()),
        ))
}

fn band(start: i32, end: i32, mapped: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingQualityQuantizationBand")
            .value("start", TestValue::Int32(start))
            .value("end", TestValue::Int32(end))
            .value("mappedValue", TestValue::Int32(mapped)),
    )
}

fn quantization(guid: u128, name: &str, bands: Vec<TestValue>) -> TestRecord {
    TestRecord::new(
        guid,
        "CraftingQualityQuantizationRecord",
        format!("CraftingQualityQuantizationRecord.{name}"),
        format!("{QUANTIZATIONS}{}.xml", name.to_ascii_lowercase()),
    )
    .value(
        "qualityQuantization",
        TestValue::Struct(
            TestInstance::new("CraftingQualityQuantization")
                .value("bands", TestValue::Array(bands)),
        ),
    )
}

fn data(link: Option<u128>) -> TestValue {
    let reference = link.map_or(TestValue::Null, TestValue::Reference);
    TestValue::Struct(
        TestInstance::new("ResourceTypeCraftingData")
            .value("name", TestValue::text("Default"))
            .value(
                "qualityQuantization",
                TestValue::Struct(
                    TestInstance::new("CraftingQualityQuantization_RecordRef")
                        .value("qualityQuantizationRecord", reference),
                ),
            ),
    )
}

fn resource(guid: u128, name: &str, properties: Vec<TestValue>) -> TestRecord {
    TestRecord::new(
        guid,
        "ResourceType",
        format!("ResourceType.{name}"),
        format!("{RESOURCES}.xml"),
    )
    .value("properties", TestValue::Array(properties))
}

fn database() -> DataForgeBuilder {
    let steps = || vec![band(0, 399, 264), band(400, 999, 553), band(1000, 1000, 1000)];
    structures(DataForgeBuilder::new().version(8))
        .record(quantization(GOLD, "Quantization_Gold", steps()))
        .record(quantization(IRON, "Quantization_Iron", vec![band(0, 1000, 500)]))
        .record(quantization(ORPHAN, "Quantization_Orphan", steps()))
        .record(quantization(ANOTHER, "Quantization_Another", steps()))
        .record(quantization(LEFTOVER, "Quantization_TEMPLATE", steps()))
        .record(quantization(NO_BANDS, "Quantization_NoBands", Vec::new()))
        .record(quantization(
            REVERSED,
            "Quantization_Reversed",
            vec![band(0, 399, 264), band(800, 400, 553)],
        ))
        .record(resource(ORE_GOLD, "Ore_Gold", vec![data(Some(GOLD))]))
        .record(resource(
            SHARED_GOLD,
            "Gold",
            vec![data(Some(GOLD)), data(Some(GOLD))],
        ))
        .record(resource(
            TWO_PROPERTIES,
            "Raw_Gold",
            vec![data(None), data(Some(GOLD))],
        ))
        .record(resource(ORE_IRON, "Ore_Iron", vec![data(Some(IRON))]))
        .record(resource(PLAIN, "Argon", Vec::new()))
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(TestEntry::zstd(
            "Data\\Localization\\english\\global.ini",
            GlobalIniBuilder::new().build(),
        ))
        .write_to(&path)?;
    Ok(path)
}
