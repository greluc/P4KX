// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Crafters with modifiers, category rules and process types, built from invented structs.

use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestInstance, TestRecord, TestStruct, TestType, TestValue,
};

pub(crate) const MODIFIED: u128 = 0x7000_0000_0000_4000_8000_0000_0000_000c;
pub(crate) const ODD: u128 = 0x7000_0000_0000_4000_8000_0000_0000_000d;
pub(crate) const BROKEN: u128 = 0x7000_0000_0000_4000_8000_0000_0000_000e;
pub(crate) const CATEGORY_A: u128 = 0x7100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const CATEGORY_B: u128 = 0x7100_0000_0000_4000_8000_0000_0000_0002;
const NO_CATEGORY: u128 = 0x7100_0000_0000_4000_8000_0000_0000_00ff;

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

/// Adds the polymorphic structs of the modifiers and the category rules to `builder`.
pub(super) fn structures(builder: DataForgeBuilder) -> DataForgeBuilder {
    [
        TestStruct::new("BlueprintCategoryRecord"),
        TestStruct::new("CraftingModifierType"),
        TestStruct::new("CraftingModifierType_Time")
            .parent("CraftingModifierType")
            .field("timeMultiplier", TestType::Single),
        TestStruct::new("CraftingModifierType_Yield")
            .parent("CraftingModifierType")
            .field("yieldMultiplier", TestType::Single),
        TestStruct::new("CraftingModifierCondition"),
        TestStruct::new("CraftingModifierCondition_BlueprintCategory")
            .parent("CraftingModifierCondition")
            .array("categories", TestType::Reference),
        TestStruct::new("CraftingModifierCondition_Daytime")
            .parent("CraftingModifierCondition")
            .field("hour", TestType::Int32),
        TestStruct::new("CraftingModifierParams")
            .field("modifierType", pointer("CraftingModifierType"))
            .field("modifierCondition", pointer("CraftingModifierCondition")),
        TestStruct::new("BlueprintCategoryAvailability"),
        TestStruct::new("BlueprintCategoryAvailability_Whitelist")
            .parent("BlueprintCategoryAvailability")
            .array("categories", TestType::Reference),
        TestStruct::new("BlueprintCategoryAvailability_Blacklist")
            .parent("BlueprintCategoryAvailability")
            .array("categories", TestType::Reference),
    ]
    .into_iter()
    .fold(builder, DataForgeBuilder::structure)
}

fn time(multiplier: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingModifierType_Time")
            .value("timeMultiplier", TestValue::Single(multiplier)),
    )
}

fn category_condition(categories: &[u128]) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingModifierCondition_BlueprintCategory")
            .value("categories", references(categories)),
    )
}

fn references(categories: &[u128]) -> TestValue {
    TestValue::Array(
        categories
            .iter()
            .copied()
            .map(TestValue::Reference)
            .collect(),
    )
}

fn modifier(kind: TestValue, condition: TestValue) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CraftingModifierParams")
            .value("modifierType", kind)
            .value("modifierCondition", condition),
    )
}

fn whitelist(categories: &[u128]) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BlueprintCategoryAvailability_Whitelist")
            .value("categories", references(categories)),
    )
}

fn flags(flags: &[bool]) -> TestValue {
    TestValue::Array(flags.iter().copied().map(TestValue::Bool).collect())
}

fn queue(availability: TestValue, process_types: TestValue) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CrafterQueue")
            .value("debugName", TestValue::text("Queue"))
            .value("maxJobsInProgress", TestValue::Int32(1))
            .value("maxJobsWaiting", TestValue::Int32(1))
            .value("blueprintCategoryAvailability", availability)
            .value("processTypeAvailability", process_types),
    )
}

fn crafter(time: f32, queues: Vec<TestValue>, modifiers: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("CrafterComponentParams")
            .value("maxJobsInHistory", TestValue::Int32(3))
            .value("baseCraftingTime", TestValue::Single(time))
            .value("queues", TestValue::Array(queues))
            .value("modifiers", TestValue::Array(modifiers)),
    )
}

fn entity(guid: u128, class: &str, size: i32, component: TestValue) -> TestEntity {
    TestEntity::new(
        guid,
        class,
        format!("libs/foundry/records/entities/basebuilding/interactables/{class}.xml"),
    )
    .attach_type("Crafter")
    .sub_type("Fabricator")
    .size(size)
    .component(component)
}

/// Adds a crafter with a time modifier under a category condition, one with a modifier type, a
/// condition and an availability the export does not know, and one with an unreadable value.
pub(super) fn records(builder: DataForgeBuilder) -> DataForgeBuilder {
    let modified = crafter(
        2.5,
        vec![queue(
            whitelist(&[CATEGORY_A, CATEGORY_B]),
            flags(&[true, false, true]),
        )],
        vec![
            modifier(time(0.75), category_condition(&[CATEGORY_B])),
            modifier(time(0.5), TestValue::Null),
        ],
    );
    let odd = crafter(
        1.0,
        vec![queue(
            TestValue::Struct(
                TestInstance::new("BlueprintCategoryAvailability_Blacklist")
                    .value("categories", references(&[CATEGORY_A])),
            ),
            flags(&[]),
        )],
        vec![
            modifier(
                TestValue::Struct(
                    TestInstance::new("CraftingModifierType_Yield")
                        .value("yieldMultiplier", TestValue::Single(2.0)),
                ),
                category_condition(&[CATEGORY_A]),
            ),
            modifier(
                time(0.9),
                TestValue::Struct(
                    TestInstance::new("CraftingModifierCondition_Daytime")
                        .value("hour", TestValue::Int32(12)),
                ),
            ),
        ],
    );
    let broken = crafter(
        1.0,
        vec![queue(whitelist(&[NO_CATEGORY]), flags(&[]))],
        vec![
            modifier(TestValue::Null, TestValue::Null),
            modifier(
                TestValue::Struct(
                    TestInstance::new("CraftingModifierType_Time")
                        .value("timeMultiplier", TestValue::Single(f32::NAN)),
                ),
                TestValue::Null,
            ),
        ],
    );
    builder
        .record(category(CATEGORY_A, "category_a"))
        .record(category(CATEGORY_B, "category_b"))
        .record(entity(MODIFIED, "modified", 2, modified).record())
        .record(entity(ODD, "odd", 1, odd).record())
        .record(entity(BROKEN, "broken", 1, broken).record())
}

fn category(guid: u128, class: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "BlueprintCategoryRecord",
        format!("BlueprintCategoryRecord.{class}"),
        "libs/foundry/records/crafting/blueprintcategories/blueprintcategorydatabase.xml",
    )
}
