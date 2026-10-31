// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestLoadout, TestRecord, TestValue};

/// Returns the Record of type `SLoadoutAssortment` named `name` with the groups `groups`, each a
/// group name and its alternative Loadouts in order.
///
/// Its record path is `libs/foundry/records/loadoutkits/<name>`; the database needs
/// [`entity_schema`](crate::entity_schema).
#[must_use]
pub fn assortment_record(
    guid: u128,
    name: &str,
    groups: &[(&str, Vec<TestLoadout>)],
) -> TestRecord {
    let group = |(group, loadouts): &(&str, Vec<TestLoadout>)| {
        TestValue::Struct(
            TestInstance::new("SGroupedLoadouts")
                .value("Group", TestValue::text(*group))
                .value(
                    "Loadouts",
                    TestValue::Array(loadouts.iter().map(TestLoadout::value).collect()),
                ),
        )
    };
    TestRecord::new(
        guid,
        "SLoadoutAssortment",
        format!("SLoadoutAssortment.{name}"),
        format!("libs/foundry/records/loadoutkits/{name}.xml"),
    )
    .value(
        "GroupedLoadouts",
        TestValue::Array(groups.iter().map(group).collect()),
    )
}
