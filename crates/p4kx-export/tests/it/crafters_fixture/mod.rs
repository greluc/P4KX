// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic Archive with crafting stations, a development leftover, an entity without a
//! crafter component and an item and a commodity that carry one.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue,
};

pub(crate) const BENCH: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const FABRICATOR: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0002;
const TEMPLATE: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0003;
const PLAIN: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0004;
const CRAFTING_ITEM: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0005;
const CRAFTING_COMMODITY: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0006;
const PLAIN_TEMPLATE: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0007;

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn queue(name: &str, in_progress: i32, waiting: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CrafterQueue")
            .value("debugName", TestValue::text(name))
            .value("maxJobsInProgress", TestValue::Int32(in_progress))
            .value("maxJobsWaiting", TestValue::Int32(waiting)),
    )
}

fn crafter(history: i32, queues: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CrafterComponentParams")
            .value("maxJobsInHistory", TestValue::Int32(history))
            .value("queues", TestValue::Array(queues)),
    )
}

fn other() -> TestValue {
    TestValue::Struct(TestInstance::new("OtherComponentParams").value("size", TestValue::Int32(1)))
}

fn entity(guid: u128, class: &str, path: &str, components: Vec<TestValue>) -> TestRecord {
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/{path}.xml"),
    )
    .value("Components", TestValue::Array(components))
}

fn structures() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(8)
        .structure(
            TestStruct::new("EntityClassDefinition")
                .array("Components", pointer("DataForgeComponentParams")),
        )
        .structure(TestStruct::new("DataForgeComponentParams"))
        .structure(
            TestStruct::new("OtherComponentParams")
                .parent("DataForgeComponentParams")
                .field("size", TestType::Int32),
        )
        .structure(
            TestStruct::new("CrafterQueue")
                .field("debugName", TestType::String)
                .field("maxJobsInProgress", TestType::Int32)
                .field("maxJobsWaiting", TestType::Int32),
        )
        .structure(
            TestStruct::new("CrafterComponentParams")
                .parent("DataForgeComponentParams")
                .field("maxJobsInHistory", TestType::Int32)
                .array("queues", TestType::Class("CrafterQueue".to_owned())),
        )
}

fn database() -> DataForgeBuilder {
    structures()
        .record(entity(
            BENCH,
            "bench",
            "basebuilding/interactables/bench",
            vec![
                other(),
                crafter(
                    10,
                    vec![queue("Fabricator", 2, 4), queue("Workbench", 4, 3)],
                ),
            ],
        ))
        .record(entity(
            FABRICATOR,
            "fabricator",
            "basebuilding/interactables/fabricator",
            vec![crafter(0, Vec::new())],
        ))
        .record(entity(
            TEMPLATE,
            "bench_template",
            "basebuilding/interactables/bench_template",
            vec![crafter(10, vec![queue("Fabricator", 2, 4)])],
        ))
        .record(entity(
            PLAIN_TEMPLATE,
            "plain_template",
            "basebuilding/interactables/plain_template",
            vec![other()],
        ))
        .record(entity(
            PLAIN,
            "plain",
            "basebuilding/interactables/plain",
            vec![other()],
        ))
        .record(entity(
            CRAFTING_ITEM,
            "crafting_item",
            "scitem/misc/crafting_item",
            vec![crafter(5, vec![queue("Item queue", 1, 1)])],
        ))
        .record(entity(
            CRAFTING_COMMODITY,
            "crafting_commodity",
            "commodities/crafting_commodity",
            vec![crafter(6, vec![queue("Commodity queue", 3, 2)])],
        ))
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .write_to(&path)?;
    Ok(path)
}
