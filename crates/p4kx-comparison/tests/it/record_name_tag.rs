// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A different Record name or Record tag is a change of the Record.

use std::error::Error;

use crate::forge_pair::ForgePair;
use p4kx_comparison::RecordChangeKind;
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0001;

fn database(version: u32, name: &str, tag: &str) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(version)
        .structure(TestStruct::new("Ship").field("speed", TestType::Int32))
        .record(
            TestRecord::new(GUID, "Ship", name, "libs/ships/one.xml")
                .tag(tag)
                .value("speed", TestValue::Int32(1)),
        )
}

type Found = Vec<(RecordChangeKind, Option<String>, Option<String>)>;

fn changes(old: &DataForgeBuilder, new: &DataForgeBuilder) -> Result<Found, Box<dyn Error>> {
    ForgePair::new(old, new)?.compare_records(|comparison| {
        comparison
            .record_changes()
            .iter()
            .map(|change| {
                let tag = |side: Option<p4kx_comparison::RecordState<'_>>| {
                    side.and_then(|state| state.tag().map(str::to_owned))
                };
                (
                    change.kind(),
                    tag(change.old_record()),
                    tag(change.new_record()),
                )
            })
            .collect()
    })
}

#[test]
fn a_changed_tag_alone_is_a_changed_record() -> TestResult {
    let found = changes(
        &database(8, "Ship.One", "SystemsDesign"),
        &database(8, "Ship.One", "Vehicles"),
    )?;

    assert_eq!(
        found,
        [(
            RecordChangeKind::Changed,
            Some("SystemsDesign".to_owned()),
            Some("Vehicles".to_owned())
        )],
        "only the tag differs"
    );
    Ok(())
}

#[test]
fn a_changed_name_alone_is_a_changed_record() -> TestResult {
    let found = changes(
        &database(8, "Ship.One", "SystemsDesign"),
        &database(8, "Ship.Two", "SystemsDesign"),
    )?;

    assert_eq!(found.len(), 1, "only the name differs: {found:?}");
    Ok(())
}

#[test]
fn equal_names_and_tags_are_no_change() -> TestResult {
    let found = changes(
        &database(8, "Ship.One", "SystemsDesign"),
        &database(8, "Ship.One", "SystemsDesign"),
    )?;

    assert!(found.is_empty(), "nothing differs: {found:?}");
    Ok(())
}
