// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A Record that cannot be read is reported on its own and the Comparison goes on.

use std::error::Error;

use crate::forge_pair::ForgePair;
use p4kx_comparison::{DataForgeError, RecordChangeKind, RecordSide};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const BROKEN: u128 = 0x4000_0000_0000_4000_8000_0000_0000_0001;
const TUNED: u128 = 0x4000_0000_0000_4000_8000_0000_0000_0002;

fn ship(declared_size: Option<u32>) -> TestStruct {
    let ship = TestStruct::new("Ship").field("speed", TestType::Int32);
    match declared_size {
        Some(size) => ship.declared_size(size),
        None => ship,
    }
}

fn database(broken_layout: bool, tuned_speed: i32) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(ship(None))
        .structure(
            TestStruct::new("Hull")
                .field("armor", TestType::Int32)
                .declared_size(if broken_layout { 8 } else { 4 }),
        )
        .record(TestRecord::new(
            BROKEN,
            "Hull",
            "Hull.Broken",
            "libs/ships/broken.xml",
        ))
        .record(
            TestRecord::new(TUNED, "Ship", "Ship.Tuned", "libs/ships/tuned.xml")
                .value("speed", TestValue::Int32(tuned_speed)),
        )
}

#[test]
fn a_record_whose_tree_cannot_be_read_is_reported_with_the_reason_and_the_rest_compared()
-> TestResult {
    let pair = ForgePair::new(&database(false, 1), &database(true, 2))?;

    let (changes, counts) = pair.compare_records(|comparison| {
        let changes: Vec<_> = comparison
            .record_changes()
            .iter()
            .map(|change| {
                (
                    change.kind(),
                    change.path().to_owned(),
                    change
                        .errors()
                        .iter()
                        .map(|error| (error.side(), error.error().clone()))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        (changes, comparison.record_counts())
    })?;

    assert_eq!(
        changes,
        [
            (
                RecordChangeKind::Unreadable,
                "libs/ships/broken".to_owned(),
                vec![(
                    RecordSide::New,
                    DataForgeError::StructLayout {
                        struct_index: 1,
                        declared: 8,
                        fields: 4
                    }
                )]
            ),
            (
                RecordChangeKind::Changed,
                "libs/ships/tuned".to_owned(),
                vec![]
            ),
        ],
        "the unreadable Record with its reason, the other one compared"
    );
    assert_eq!(
        (counts.unreadable, counts.changed, counts.total()),
        (1, 1, 2),
        "counted per kind"
    );
    Ok(())
}

#[test]
fn an_unreadable_record_counts_as_a_difference() -> TestResult {
    let pair = ForgePair::new(&database(true, 1), &database(true, 1))?;

    let (kinds, differs) = pair.compare_records(|comparison| {
        (
            comparison
                .record_changes()
                .iter()
                .map(|change| (change.kind(), change.errors().len()))
                .collect::<Vec<_>>(),
            comparison.has_differences(),
        )
    })?;

    assert_eq!(
        kinds,
        [(RecordChangeKind::Unreadable, 2)],
        "both sides unreadable"
    );
    assert!(differs, "an unreadable Record is a difference");
    Ok(())
}
