//! A synthetic Archive with mineable elements and refining processes.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestRecord, TestStruct,
    TestType, TestValue,
};

const AGRICIUM: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0003;
const AGRICIUM_DEPOSIT: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const BEXALITE: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0002;
const LEFTOVER_ELEMENT: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0004;
pub(crate) const FAST_CAREFUL: u128 = 0x4000_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const SLOW_WASTEFUL: u128 = 0x4000_0000_0000_4000_8000_0000_0000_0001;
const LEFTOVER_PROCESS: u128 = 0x4000_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const SHARED_RESOURCE: u128 = 0x5000_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const OTHER_RESOURCE: u128 = 0x5000_0000_0000_4000_8000_0000_0000_0002;

const ELEMENTS: &str = "libs/foundry/records/mining/mineableelements/";
const PROCESSES: &str = "libs/foundry/records/refiningprocess/";

fn structures(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder
        .structure(TestStruct::new("ResourceType"))
        .enumeration("RefiningSpeed", ["Slow", "Normal", "Fast"])
        .enumeration("RefiningQuality", ["Wasteful", "Normal", "Careful"])
        .structure(
            TestStruct::new("MineableElement")
                .field("elementInstability", TestType::Single)
                .field("elementResistance", TestType::Single)
                .field("elementOptimalWindowMidpoint", TestType::Single)
                .field("elementOptimalWindowMidpointRandomness", TestType::Single)
                .field("elementOptimalWindowThinness", TestType::Single)
                .field("elementExplosionMultiplier", TestType::Single)
                .field("elementClusterFactor", TestType::Single)
                .field("resourceType", TestType::Reference),
        )
        .structure(
            TestStruct::new("RefiningProcess")
                .field("refiningSpeed", TestType::Enum("RefiningSpeed".into()))
                .field("refiningQuality", TestType::Enum("RefiningQuality".into()))
                .field("processName", TestType::Locale),
        )
}

fn resource(guid: u128, name: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "ResourceType",
        format!("ResourceType.{name}"),
        format!("libs/foundry/records/resourcetypedatabase/{name}.xml"),
    )
}

fn element(guid: u128, name: &str, instability: f32, resource: Option<u128>) -> TestRecord {
    let record = TestRecord::new(
        guid,
        "MineableElement",
        format!("MineableElement.{name}"),
        format!("{ELEMENTS}{name}.xml"),
    )
    .value("elementInstability", TestValue::Single(instability))
    .value("elementResistance", TestValue::Single(-0.5))
    .value("elementOptimalWindowMidpoint", TestValue::Single(0.5))
    .value(
        "elementOptimalWindowMidpointRandomness",
        TestValue::Single(0.25),
    )
    .value("elementOptimalWindowThinness", TestValue::Single(2.0))
    .value("elementExplosionMultiplier", TestValue::Single(4.0))
    .value("elementClusterFactor", TestValue::Single(0.125));
    match resource {
        Some(resource) => record.value("resourceType", TestValue::Reference(resource)),
        None => record,
    }
}

fn process(guid: u128, name: &str, speed: &str, quality: &str, key: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "RefiningProcess",
        format!("RefiningProcess.{name}"),
        format!("{PROCESSES}{name}.xml"),
    )
    .value("refiningSpeed", TestValue::text(speed))
    .value("refiningQuality", TestValue::text(quality))
    .value("processName", TestValue::text(key))
}

fn database() -> DataForgeBuilder {
    structures(DataForgeBuilder::new().version(8))
        .record(resource(SHARED_RESOURCE, "shared"))
        .record(resource(OTHER_RESOURCE, "other"))
        .record(element(
            AGRICIUM,
            "agricium_ore",
            350.0,
            Some(SHARED_RESOURCE),
        ))
        .record(element(
            AGRICIUM_DEPOSIT,
            "agricium_deposit",
            425.0,
            Some(SHARED_RESOURCE),
        ))
        .record(element(BEXALITE, "bexalite_ore", 0.0, Some(OTHER_RESOURCE)))
        .record(element(LEFTOVER_ELEMENT, "mineable_test", 1.0, None))
        .record(process(
            FAST_CAREFUL,
            "processingtype-fastcareful",
            "Fast",
            "Careful",
            "@refinery_ui_ProcessingType_FastCareful",
        ))
        .record(process(
            SLOW_WASTEFUL,
            "processingtype-slowwasteful",
            "Slow",
            "Wasteful",
            "@refinery_ui_ProcessingType_SlowWasteful",
        ))
        .record(process(
            LEFTOVER_PROCESS,
            "processingtype-dev",
            "Normal",
            "Normal",
            "@LOC_PLACEHOLDER",
        ))
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(TestEntry::zstd(
            "Data\\Localization\\english\\global.ini",
            GlobalIniBuilder::new()
                .text("refinery_ui_ProcessingType_FastCareful", "Fast, Careful")
                .build(),
        ))
        .write_to(&path)?;
    Ok(path)
}
