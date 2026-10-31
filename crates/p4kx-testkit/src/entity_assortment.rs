// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestLoadout, TestRecord, TestRequirement, TestValue};

/// One group of a synthetic assortment: its name, its Requirement and its alternative Loadouts in
/// order.
pub type TestGroup<'a> = (&'a str, Option<TestRequirement>, Vec<TestLoadout>);

/// Returns the Record of type `SLoadoutAssortment` named `name` with the groups `groups`, each a
/// group name and its alternative Loadouts in order, none with a Requirement.
///
/// Its record path is `libs/foundry/records/loadoutkits/<name>`; the database needs
/// [`entity_schema`](crate::entity_schema).
#[must_use]
pub fn assortment_record(
    guid: u128,
    name: &str,
    groups: &[(&str, Vec<TestLoadout>)],
) -> TestRecord {
    let groups: Vec<TestGroup<'_>> = groups
        .iter()
        .map(|(group, loadouts)| (*group, None, loadouts.clone()))
        .collect();
    assortment_record_requiring(guid, name, None, &groups)
}

/// Like [`assortment_record`], with the Requirement of the assortment and of each group.
#[must_use]
pub fn assortment_record_requiring(
    guid: u128,
    name: &str,
    requirement: Option<&TestRequirement>,
    groups: &[TestGroup<'_>],
) -> TestRecord {
    let group = |(group, requirement, loadouts): &TestGroup<'_>| {
        let mut instance = TestInstance::new("SGroupedLoadouts")
            .value("Group", TestValue::text(*group))
            .value(
                "Loadouts",
                TestValue::Array(loadouts.iter().map(TestLoadout::value).collect()),
            );
        if let Some(requirement) = requirement {
            instance = instance.value("Requirement", requirement.value());
        }
        TestValue::Struct(instance)
    };
    let record = TestRecord::new(
        guid,
        "SLoadoutAssortment",
        format!("SLoadoutAssortment.{name}"),
        format!("libs/foundry/records/loadoutkits/{name}.xml"),
    )
    .value(
        "GroupedLoadouts",
        TestValue::Array(groups.iter().map(group).collect()),
    );
    match requirement {
        Some(requirement) => record.value("Requirement", requirement.value()),
        None => record,
    }
}
