// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The places, factions and blueprint rewards that the contract gaps of the catalog read.

use p4kx_testkit::{TestInstance, TestValue};

use super::lookups::{amount, amount_of, flags, legacy};
use super::prerequisites::{location, standing_prerequisite};
use super::records::results;
use super::{
    FACTION_SCOPE, LOSS, OBJ_MISSING, OBJ_STAR, RANK_FIFTY, RIVAL, STRAY_FACTION, SUB_TWO,
};

const SUCCESS: [bool; 5] = [true, false, false, false, false];

/// Locations that name a star, an object without a Record and the star again.
pub(super) fn odd_locations() -> TestValue {
    TestValue::Array(vec![
        location(OBJ_STAR),
        location(OBJ_MISSING),
        location(OBJ_STAR),
    ])
}

/// A success that costs standing with a rival and, after it, grants standing.
pub(super) fn rival_then_gain() -> TestValue {
    results(vec![legacy(
        SUCCESS,
        vec![
            amount_of(RIVAL, FACTION_SCOPE, LOSS),
            amount(FACTION_SCOPE, RANK_FIFTY),
        ],
    )])
}

/// A success that only costs standing with a rival.
pub(super) fn rival_only() -> TestValue {
    results(vec![legacy(
        SUCCESS,
        vec![amount_of(RIVAL, FACTION_SCOPE, LOSS)],
    )])
}

/// One reputation prerequisite that is a requirement, about a faction outside the faction folder.
pub(super) fn requirement() -> TestValue {
    TestValue::Array(vec![TestValue::Struct(
        TestInstance::new("ContractPrerequisite_Reputation")
            .value("exclude", TestValue::Bool(false))
            .value("factionReputation", TestValue::Reference(STRAY_FACTION)),
    )])
}

/// Parameter overrides with an enabled and a disabled temporary blueprint modifier.
pub(super) fn modifier_overrides() -> TestValue {
    let modifier = |enabled: bool, pool: u128| {
        TestValue::Struct(
            TestInstance::new("MissionModifier_TemporaryBlueprint")
                .value("enabled", TestValue::Bool(enabled))
                .value("blueprintPool", TestValue::Reference(pool)),
        )
    };
    TestValue::Struct(TestInstance::new("ContractParamOverrides").value(
        "modifierOverrides",
        TestValue::Array(vec![
            modifier(false, POOL_UNREACHED),
            modifier(true, POOL_MODIFIER),
        ]),
    ))
}

pub(super) const POOL_CONTRACT: u128 = 0xe400_0000_0000_4000_8000_0000_0000_0001;
pub(super) const POOL_UNREACHED: u128 = 0xe400_0000_0000_4000_8000_0000_0000_0002;
const POOL_MODIFIER: u128 = 0xe400_0000_0000_4000_8000_0000_0000_0004;
const POOL_SUB: u128 = 0xe400_0000_0000_4000_8000_0000_0000_0003;

/// A `BlueprintRewards` result that fires on success when `on_success`, else on the second
/// outcome.
pub(super) fn blueprint(on_success: bool, pool: u128, chance: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BlueprintRewards")
            .value("chance", TestValue::Single(chance))
            .value(
                "missionResults",
                flags([on_success, !on_success, false, false, false]),
            )
            .value("blueprintPool", TestValue::Reference(pool)),
    )
}

/// The second sub contract of the first contract, with results of its own.
pub(super) fn sub_two() -> TestValue {
    TestValue::Struct(
        TestInstance::new("SubContract")
            .value("id", TestValue::Guid(SUB_TWO))
            .value("debugName", TestValue::text("Sub_Contract"))
            .value(
                "contractResults",
                results(vec![blueprint(true, POOL_SUB, 1.0)]),
            ),
    )
}

/// One reputation prerequisite that is an exclusion.
pub(super) fn exclusion() -> TestValue {
    TestValue::Array(vec![standing_prerequisite(true, None)])
}
