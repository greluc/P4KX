// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic Archive with ships that carry insurance data, ships that carry none and an item.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue,
};

pub(crate) const INSURED: u128 = 0x6000_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const UNINSURED: u128 = 0x6000_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const BARE: u128 = 0x6000_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const ITEM: u128 = 0x6000_0000_0000_4000_8000_0000_0000_0004;
pub(crate) const RECOVERY: u128 = 0x6000_0000_0000_4000_8000_0000_0000_0005;

fn structures(builder: DataForgeBuilder) -> DataForgeBuilder {
    builder
        .structure(TestStruct::new("StaticEntityClassData"))
        .structure(
            TestStruct::new("ShipInsuranceParams")
                .field("shipEntityClassName", TestType::String)
                .field("baseWaitTimeMinutes", TestType::Single)
                .field("mandatoryWaitTimeMinutes", TestType::Single)
                .field("baseExpeditingFee", TestType::UInt32),
        )
        .structure(
            TestStruct::new("SEntityInsuranceProperties")
                .parent("StaticEntityClassData")
                .field("restoreOnClaim", TestType::Bool)
                .field(
                    "shipInsuranceParams",
                    TestType::Class("ShipInsuranceParams".to_owned()),
                ),
        )
        .structure(TestStruct::new("EntityClassDefinition").array(
            "StaticEntityClassData",
            TestType::StrongPointer("StaticEntityClassData".to_owned()),
        ))
        .structure(
            TestStruct::new("ItemRecoveryEscalatingClaimsParams")
                .field("escalatingClaimChainLimit", TestType::Int32)
                .field("escalationTimerMultiplier", TestType::Single),
        )
        .structure(
            TestStruct::new("ItemRecoveryOverrideGroupDef")
                .field("claimCostMultiplier", TestType::Single)
                .array("classes", TestType::Reference),
        )
        .structure(
            TestStruct::new("ItemRecoveryEconomyParams")
                .field("claimCostMultiplier", TestType::Single)
                .field("maxExpeditedClaimCostMultiplier", TestType::Single)
                .field("expeditedDeliverySecondsPerAUEC", TestType::Single)
                .field(
                    "escalatingClaimsParams",
                    TestType::Class("ItemRecoveryEscalatingClaimsParams".to_owned()),
                )
                .array(
                    "overrides",
                    TestType::Class("ItemRecoveryOverrideGroupDef".to_owned()),
                ),
        )
        .structure(TestStruct::new("ItemRecoveryConfigurationParams").field(
            "economyParams",
            TestType::Class("ItemRecoveryEconomyParams".to_owned()),
        ))
}

fn recovery() -> TestRecord {
    let group = |multiplier: f32, classes: Vec<u128>| {
        TestValue::Struct(
            TestInstance::new("ItemRecoveryOverrideGroupDef")
                .value("claimCostMultiplier", TestValue::Single(multiplier))
                .value(
                    "classes",
                    TestValue::Array(classes.into_iter().map(TestValue::Reference).collect()),
                ),
        )
    };
    TestRecord::new(
        RECOVERY,
        "ItemRecoveryConfigurationParams",
        "ItemRecoveryConfigurationParams.GlobalItemRecovery",
        "libs/foundry/records/entitlementpolicies/globalitemrecovery.xml",
    )
    .value(
        "economyParams",
        TestValue::Struct(
            TestInstance::new("ItemRecoveryEconomyParams")
                .value("claimCostMultiplier", TestValue::Single(0.003))
                .value("maxExpeditedClaimCostMultiplier", TestValue::Single(0.0003))
                .value("expeditedDeliverySecondsPerAUEC", TestValue::Single(5e-7))
                .value(
                    "escalatingClaimsParams",
                    TestValue::Struct(
                        TestInstance::new("ItemRecoveryEscalatingClaimsParams")
                            .value("escalatingClaimChainLimit", TestValue::Int32(1))
                            .value("escalationTimerMultiplier", TestValue::Single(1.0)),
                    ),
                )
                .value(
                    "overrides",
                    TestValue::Array(vec![group(1.0, vec![INSURED, BARE]), group(0.6, vec![])]),
                ),
        ),
    )
}

fn params(class: &str, claim: f32, mandatory: f32, fee: u32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ShipInsuranceParams")
            .value("shipEntityClassName", TestValue::text(class))
            .value("baseWaitTimeMinutes", TestValue::Single(claim))
            .value("mandatoryWaitTimeMinutes", TestValue::Single(mandatory))
            .value("baseExpeditingFee", TestValue::UInt32(fee)),
    )
}

fn insurance(restore: bool, params: TestValue) -> TestValue {
    TestValue::Struct(
        TestInstance::new("SEntityInsuranceProperties")
            .value("restoreOnClaim", TestValue::Bool(restore))
            .value("shipInsuranceParams", params),
    )
}

fn entity(guid: u128, class: &str, path: &str, data: Vec<TestValue>) -> TestRecord {
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/{path}.xml"),
    )
    .value("StaticEntityClassData", TestValue::Array(data))
}

fn database() -> DataForgeBuilder {
    structures(DataForgeBuilder::new().version(8))
        .record(entity(
            INSURED,
            "AEGS_Insured",
            "spaceships/aegs_insured",
            vec![insurance(true, params("AEGS_Insured", 5.83, 1.94, 2335))],
        ))
        .record(entity(
            UNINSURED,
            "AEGS_Uninsured",
            "spaceships/aegs_uninsured",
            Vec::new(),
        ))
        .record(entity(
            BARE,
            "AEGS_Bare",
            "groundvehicles/aegs_bare",
            vec![insurance(false, params("", 0.0, 0.0, 0))],
        ))
        .record(entity(
            ITEM,
            "item_with_insurance",
            "scitem/misc/item_with_insurance",
            vec![insurance(true, params("Item", 1.0, 1.0, 1))],
        ))
        .record(recovery())
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .write_to(&path)?;
    Ok(path)
}
