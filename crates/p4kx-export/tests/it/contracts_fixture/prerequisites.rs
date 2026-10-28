// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The prerequisites a contract, a sub contract or a handler states.

use p4kx_testkit::{TestInstance, TestValue};

use super::records::reference;
use super::{FACTION, FACTION_SCOPE};

pub(super) fn locality(guid: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractPrerequisite_Locality")
            .value("localityAvailable", reference(guid)),
    )
}

pub(super) fn location(guid: u128) -> TestValue {
    TestValue::Struct(
        TestInstance::new("ContractPrerequisite_Location")
            .value("locationAvailable", reference(guid)),
    )
}

/// A reputation prerequisite about the fixture faction and scope, with the standing range when
/// `standings` gives one.
pub(super) fn standing_prerequisite(exclude: bool, standings: Option<(u128, u128)>) -> TestValue {
    let mut prerequisite = TestInstance::new("ContractPrerequisite_Reputation")
        .value("exclude", TestValue::Bool(exclude))
        .value("factionReputation", reference(FACTION))
        .value("scope", reference(FACTION_SCOPE));
    if let Some((minimum, maximum)) = standings {
        prerequisite = prerequisite
            .value("minStanding", reference(minimum))
            .value("maxStanding", reference(maximum));
    }
    TestValue::Struct(prerequisite)
}
