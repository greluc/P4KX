// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic Archive for the contract facts: one handler with an availability, a template with
//! a contract class, contracts that override parameters at every level, and an intro contract
//! that grants the completion tag another contract requires.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue,
};

pub(crate) const BARE: u128 = 0x2100_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const OVERRIDDEN: u128 = 0x2100_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const INTRO: u128 = 0x2100_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const GATED: u128 = 0x2100_0000_0000_4000_8000_0000_0000_0004;
pub(crate) const FAILS_ONLY: u128 = 0x2100_0000_0000_4000_8000_0000_0000_0005;
pub(crate) const STRIPPED: u128 = 0x2100_0000_0000_4000_8000_0000_0000_0006;
pub(crate) const SUB: u128 = 0x2200_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const SUB_DIRECT: u128 = 0x2200_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const TAG: u128 = 0x2300_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const FAIL_TAG: u128 = 0x2300_0000_0000_4000_8000_0000_0000_0002;
const TEMPLATE: u128 = 0x2400_0000_0000_4000_8000_0000_0000_0001;
const GENERATOR: u128 = 0x2500_0000_0000_4000_8000_0000_0000_0001;

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

fn handler_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("ContractGenerator").array("generators", pointer("ContractHandler")),
        TestStruct::new("ContractHandler"),
        TestStruct::new("ContractHandler_Bare")
            .parent("ContractHandler")
            .field("debugName", TestType::String)
            .array("contracts", class("BareContract")),
        TestStruct::new("BareContract")
            .field("id", TestType::Guid)
            .field("debugName", TestType::String),
        TestStruct::new("ContractHandler_Career")
            .parent("ContractHandler")
            .field("debugName", TestType::String)
            .field("defaultAvailability", class("ContractAvailability"))
            .field("contractParams", class("ContractParamOverrides"))
            .array("contracts", class("CareerContract"))
            .array("introContracts", class("CareerContract")),
        TestStruct::new("ContractAvailability")
            .field("onceOnly", TestType::Bool)
            .field("maxPlayersPerInstance", TestType::Int32)
            .field("availableInPrison", TestType::Bool)
            .field("canReacceptAfterAbandoning", TestType::Bool)
            .field("canReacceptAfterFailing", TestType::Bool)
            .field("hideInMobiGlas", TestType::Bool)
            .array("prerequisites", pointer("ContractPrerequisite")),
    ]
}

fn prerequisite_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("ContractPrerequisite"),
        TestStruct::new("ContractPrerequisite_CompletedContractTags")
            .parent("ContractPrerequisite")
            .field("requiredCompletedContractTags", class("TagList")),
        TestStruct::new("TagList").array("tags", TestType::Reference),
    ]
}

fn contract_structs() -> Vec<TestStruct> {
    let params = || {
        [
            ("boolParamOverrides", "ContractBoolParam"),
            ("intParamOverrides", "ContractIntParam"),
        ]
    };
    let mut overrides = TestStruct::new("ContractParamOverrides");
    let mut sub = TestStruct::new("SubContract")
        .field("id", TestType::Guid)
        .field("debugName", TestType::String)
        .array("additionalPrerequisites", pointer("ContractPrerequisite"));
    for (field, kind) in params() {
        overrides = overrides.array(field, class(kind));
        sub = sub.array(field, class(kind));
    }
    vec![
        overrides,
        TestStruct::new("ContractBoolParam")
            .field("param", TestType::String)
            .field("value", TestType::Bool),
        TestStruct::new("ContractIntParam")
            .field("param", TestType::String)
            .field("value", TestType::Int32),
        TestStruct::new("CareerContract")
            .field("id", TestType::Guid)
            .field("debugName", TestType::String)
            .field("template", TestType::Reference)
            .field("paramOverrides", class("ContractParamOverrides"))
            .array("subContracts", class("SubContract"))
            .array("additionalPrerequisites", pointer("ContractPrerequisite"))
            .field("contractResults", class("ContractResultBlock")),
        sub,
    ]
}

fn result_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("ContractResultBlock")
            .field("timeToComplete", TestType::Single)
            .field("contractBuyInAmount", TestType::Int32)
            .array("contractResults", pointer("ContractResult")),
        TestStruct::new("ContractResult"),
        TestStruct::new("ContractResult_CompletionTags")
            .parent("ContractResult")
            .array("missionResults", class("Bool"))
            .array("completionTags", class("ContractResult_CompletionTag")),
        TestStruct::new("ContractResult_CompletionTag")
            .field("count", TestType::Int32)
            .field("tag", TestType::Reference),
        TestStruct::new("Bool").field("value", TestType::Bool),
    ]
}

fn template_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("ContractTemplate")
            .field("contractDisplayInfo", class("ContractDisplayInfo"))
            .field("contractClass", pointer("ContractClass")),
        TestStruct::new("ContractDisplayInfo").field("illegal", TestType::Bool),
        TestStruct::new("ContractClass"),
        TestStruct::new("ContractClass_Contract")
            .parent("ContractClass")
            .array("additionalParams", pointer("ContractAdditionalParams")),
        TestStruct::new("ContractAdditionalParams"),
        TestStruct::new("ActiveContractSettings")
            .parent("ContractAdditionalParams")
            .field("canBeShared", TestType::Bool),
        TestStruct::new("Tag").field("tagName", TestType::String),
    ]
}

fn flag(param: &str, value: bool) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractBoolParam")
            .value("param", TestValue::text(param))
            .value("value", TestValue::Bool(value)),
    )
}

fn number(param: &str, value: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractIntParam")
            .value("param", TestValue::text(param))
            .value("value", TestValue::Int32(value)),
    )
}

fn overrides(bools: Vec<TestValue>, ints: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractParamOverrides")
            .value("boolParamOverrides", TestValue::Array(bools))
            .value("intParamOverrides", TestValue::Array(ints)),
    )
}

fn requires(tags: &[u128]) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractPrerequisite_CompletedContractTags").value(
            "requiredCompletedContractTags",
            TestValue::Struct(TestInstance::new("TagList").value(
                "tags",
                TestValue::Array(tags.iter().map(|tag| TestValue::Reference(*tag)).collect()),
            )),
        ),
    )
}

fn grants(success: bool, tag: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractResult_CompletionTags")
            .value(
                "missionResults",
                TestValue::Array(vec![TestValue::Struct(
                    TestInstance::new("Bool").value("value", TestValue::Bool(success)),
                )]),
            )
            .value(
                "completionTags",
                TestValue::Array(vec![TestValue::Struct(
                    TestInstance::new("ContractResult_CompletionTag")
                        .value("count", TestValue::Int32(1))
                        .value("tag", TestValue::Reference(tag)),
                )]),
            ),
    )
}

fn terms(time: f32, buy_in: i32, results: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractResultBlock")
            .value("timeToComplete", TestValue::Single(time))
            .value("contractBuyInAmount", TestValue::Int32(buy_in))
            .value("contractResults", TestValue::Array(results)),
    )
}

fn contract(id: u128, name: &str) -> TestInstance {
    TestInstance::new("CareerContract")
        .value("id", TestValue::Guid(id))
        .value("debugName", TestValue::text(name))
        .value("template", TestValue::Reference(TEMPLATE))
}

fn sub_contract(id: u128, name: &str) -> TestInstance {
    TestInstance::new("SubContract")
        .value("id", TestValue::Guid(id))
        .value("debugName", TestValue::text(name))
}

fn contracts() -> Vec<TestValue> {
    let bare = contract(BARE, "Bare");
    let overridden = contract(OVERRIDDEN, "Overridden")
        .value(
            "paramOverrides",
            overrides(
                vec![flag("OnceOnly", false), flag("Illegal", true)],
                vec![number("MaxPlayersPerInstance", 4)],
            ),
        )
        .value("contractResults", terms(25.0, 1500, Vec::new()))
        .value(
            "subContracts",
            TestValue::Array(vec![
                TestValue::Struct(
                    sub_contract(SUB, "Sub")
                        .value(
                            "boolParamOverrides",
                            TestValue::Array(vec![flag("canbeshared", false)]),
                        )
                        .value(
                            "additionalPrerequisites",
                            TestValue::Array(vec![requires(&[TAG])]),
                        ),
                ),
                TestValue::Struct(sub_contract(SUB_DIRECT, "SubDirect").value(
                    "boolParamOverrides",
                    TestValue::Array(vec![flag("OnceOnly", true)]),
                )),
            ]),
        );
    let gated = contract(GATED, "Gated").value(
        "additionalPrerequisites",
        TestValue::Array(vec![requires(&[TAG, FAIL_TAG])]),
    );
    let fails_only = contract(FAILS_ONLY, "FailsOnly").value(
        "contractResults",
        terms(0.0, 0, vec![grants(false, FAIL_TAG)]),
    );
    [bare, overridden, gated, fails_only]
        .into_iter()
        .map(TestValue::Struct)
        .collect()
}

fn stripped_handler() -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractHandler_Bare")
            .value("debugName", TestValue::text("Stripped"))
            .value(
                "contracts",
                TestValue::Array(vec![TestValue::Struct(
                    TestInstance::new("BareContract")
                        .value("id", TestValue::Guid(STRIPPED))
                        .value("debugName", TestValue::text("Stripped")),
                )]),
            ),
    )
}

fn intro_contract() -> TestValue {
    TestValue::Struct(
        contract(INTRO, "Intro").value("contractResults", terms(0.0, 0, vec![grants(true, TAG)])),
    )
}

fn handler() -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractHandler_Career")
            .value("debugName", TestValue::text("Facts_Career"))
            .value(
                "defaultAvailability",
                TestValue::Struct(
                    TestInstance::new("ContractAvailability")
                        .value("onceOnly", TestValue::Bool(true))
                        .value("maxPlayersPerInstance", TestValue::Int32(1))
                        .value("canReacceptAfterAbandoning", TestValue::Bool(true))
                        .value("canReacceptAfterFailing", TestValue::Bool(false))
                        .value("hideInMobiGlas", TestValue::Bool(false))
                        .value(
                            "prerequisites",
                            TestValue::Array(vec![requires(&[FAIL_TAG])]),
                        ),
                ),
            )
            .value(
                "contractParams",
                overrides(vec![flag("HideInMobiGlas", true)], Vec::new()),
            )
            .value("contracts", TestValue::Array(contracts()))
            .value("introContracts", TestValue::Array(vec![intro_contract()])),
    )
}

fn template() -> TestRecord {
    TestRecord::new(
        TEMPLATE,
        "ContractTemplate",
        "ContractTemplate.facts",
        "libs/foundry/records/contracts/contracttemplates/facts.xml",
    )
    .value(
        "contractDisplayInfo",
        TestValue::Struct(
            TestInstance::new("ContractDisplayInfo").value("illegal", TestValue::Bool(false)),
        ),
    )
    .value(
        "contractClass",
        TestValue::Struct(TestInstance::new("ContractClass_Contract").value(
            "additionalParams",
            TestValue::Array(vec![TestValue::Struct(
                    TestInstance::new("ActiveContractSettings")
                        .value("canBeShared", TestValue::Bool(true)),
                )]),
        )),
    )
}

fn tag(guid: u128, name: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "Tag",
        format!("Tag.{name}"),
        "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml",
    )
    .value("tagName", TestValue::text(name))
}

fn database() -> DataForgeBuilder {
    let builder = [
        handler_structs(),
        contract_structs(),
        prerequisite_structs(),
        result_structs(),
        template_structs(),
    ]
    .into_iter()
    .flatten()
    .fold(
        DataForgeBuilder::new().version(8),
        DataForgeBuilder::structure,
    );
    builder
        .record(template())
        .record(tag(TAG, "intro_done"))
        .record(tag(FAIL_TAG, "never_granted"))
        .record(
            TestRecord::new(
                GENERATOR,
                "ContractGenerator",
                "ContractGenerator.facts",
                "libs/foundry/records/contracts/contractgenerator/facts.xml",
            )
            .value(
                "generators",
                TestValue::Array(vec![handler(), stripped_handler()]),
            ),
        )
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .write_to(&path)?;
    Ok(path)
}
