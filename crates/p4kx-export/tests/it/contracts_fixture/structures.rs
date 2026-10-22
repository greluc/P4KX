use p4kx_testkit::{DataForgeBuilder, TestStruct, TestType};

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

fn value(name: &str) -> TestStruct {
    TestStruct::new(name).parent("MissionPropertyValue")
}

/// Adds the structs of the fixture to `builder`.
pub(super) fn structures(builder: DataForgeBuilder) -> DataForgeBuilder {
    [generator_structs(), property_structs(), record_structs()]
        .into_iter()
        .flatten()
        .fold(builder, DataForgeBuilder::structure)
}

fn generator_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("ContractGenerator")
            .array("generators", pointer("ContractGeneratorHandler")),
        TestStruct::new("ContractGeneratorHandler"),
        TestStruct::new("ContractGeneratorHandler_Career")
            .parent("ContractGeneratorHandler")
            .field("debugName", TestType::String)
            .field("contractParams", class("ContractParamOverrides"))
            .array("contracts", class("CareerContract"))
            .array("introContracts", class("CareerContract")),
        TestStruct::new("ContractParamOverrides")
            .field("missionTypeOverride", TestType::Reference)
            .array("stringParamOverrides", class("ContractStringParam"))
            .array("propertyOverrides", class("MissionProperty")),
        TestStruct::new("ContractStringParam")
            .field("param", TestType::String)
            .field("value", TestType::Locale),
        TestStruct::new("CareerContract")
            .field("id", TestType::Guid)
            .field("debugName", TestType::String)
            .field("notForRelease", TestType::Bool)
            .field("template", TestType::Reference)
            .field("paramOverrides", class("ContractParamOverrides"))
            .array("subContracts", class("SubContract"))
            .field("contractResults", class("ContractResultBlock")),
        TestStruct::new("SubContract")
            .field("id", TestType::Guid)
            .field("paramOverrides", class("ContractParamOverrides")),
        TestStruct::new("ContractResultBlock").array("contractResults", pointer("ContractResult")),
        TestStruct::new("ContractResult"),
        TestStruct::new("ContractResult_LegacyReputation")
            .parent("ContractResult")
            .array("missionResults", class("Bool"))
            .array("contractResultReputationAmounts", class("ReputationAmount")),
        TestStruct::new("ContractResult_CompletionTags").parent("ContractResult"),
        TestStruct::new("Bool").field("value", TestType::Bool),
        TestStruct::new("ReputationAmount")
            .field("factionReputation", TestType::Reference)
            .field("reputationScope", TestType::Reference)
            .field("reward", TestType::Reference),
    ]
}

fn property_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("MissionProperty")
            .field("missionVariableName", TestType::String)
            .field("extendedTextToken", TestType::String)
            .field("value", pointer("MissionPropertyValue")),
        TestStruct::new("MissionPropertyValue"),
        value("MissionPropertyValue_StringHash")
            .array("options", class("MissionPropertyValueOption_StringHash")),
        TestStruct::new("MissionPropertyValueOption_StringHash")
            .field("textId", TestType::Locale)
            .field("weighting", TestType::Int32),
        value("MissionPropertyValue_Organization")
            .array("matchConditions", pointer("DataSetMatchCondition")),
        TestStruct::new("DataSetMatchCondition"),
        TestStruct::new("DataSetMatchCondition_SpecificOrganizationsDef")
            .parent("DataSetMatchCondition")
            .array("organizations", TestType::Reference),
        value("MissionPropertyValue_Location"),
        value("MissionPropertyValue_AIName"),
        value("MissionPropertyValue_ShipSpawnDescriptions"),
        value("MissionPropertyValue_Boolean").field("value", TestType::Bool),
    ]
}

fn record_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("ContractTemplate")
            .field("contractDisplayInfo", class("ContractDisplayInfo"))
            .array("contractProperties", class("MissionProperty")),
        TestStruct::new("ContractDisplayInfo")
            .field("type", TestType::Reference)
            .array("displayString", class("LocID")),
        TestStruct::new("LocID").field("value", TestType::Locale),
        TestStruct::new("MissionType").field("LocalisedTypeName", TestType::Locale),
        TestStruct::new("MissionOrganization")
            .field("stringVariants", class("MissionStringVariants")),
        TestStruct::new("MissionStringVariants").array("variants", class("MissionStringVariant")),
        TestStruct::new("MissionStringVariant")
            .field("tag", TestType::Reference)
            .field("string", TestType::Locale),
        TestStruct::new("FactionReputation"),
        TestStruct::new("ReputationScope"),
        TestStruct::new("ReputationReward").field("reputationAmount", TestType::Int32),
        TestStruct::new("Tag")
            .field("tagName", TestType::String)
            .field("legacyGUID", TestType::UInt32)
            .array("children", TestType::ReferenceTo("Tag".into())),
        TestStruct::new("TagDatabase").array("tags", TestType::ReferenceTo("Tag".into())),
    ]
}
