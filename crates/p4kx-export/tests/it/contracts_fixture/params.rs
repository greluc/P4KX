// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_testkit::{TestInstance, TestValue};

use super::records::reference;

pub(super) fn param(name: &str, key: &str) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractStringParam")
            .value("param", TestValue::text(name))
            .value("value", TestValue::text(key)),
    )
}

pub(super) fn overrides(
    title: Option<&str>,
    mission_type: Option<u128>,
    properties: Vec<TestValue>,
) -> TestValue {
    described(title, None, mission_type, properties)
}

pub(super) fn described(
    title: Option<&str>,
    description: Option<&str>,
    mission_type: Option<u128>,
    properties: Vec<TestValue>,
) -> TestValue {
    let mut params = TestInstance::new("ContractParamOverrides");
    if let Some(guid) = mission_type {
        params = params.value("missionTypeOverride", reference(guid));
    }
    let strings = [("Title", title), ("Description", description)]
        .into_iter()
        .filter_map(|(name, key)| key.map(|key| param(name, key)))
        .collect();
    TestValue::Struct(
        params
            .value("stringParamOverrides", TestValue::Array(strings))
            .value("propertyOverrides", TestValue::Array(properties)),
    )
}
