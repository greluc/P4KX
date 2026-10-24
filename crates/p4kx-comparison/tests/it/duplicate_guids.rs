// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Records that share a GUID within one DataForge are joined in definition order.

use std::error::Error;

use crate::forge_pair::ForgePair;
use p4kx_comparison::RecordChangeKind;
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType};

type TestResult = Result<(), Box<dyn Error>>;

const SHARED: u128 = 0x5000_0000_0000_4000_8000_0000_0000_0001;

fn database(paths: &[&str]) -> DataForgeBuilder {
    paths.iter().fold(
        DataForgeBuilder::new().structure(TestStruct::new("Ship").field("speed", TestType::Int32)),
        |builder, path| builder.record(TestRecord::new(SHARED, "Ship", "Ship.Twin", *path)),
    )
}

#[test]
fn records_sharing_a_guid_are_paired_in_definition_order() -> TestResult {
    let pair = ForgePair::new(
        &database(&[
            "libs/ships/first.xml",
            "libs/ships/second.xml",
            "libs/ships/third.xml",
        ]),
        &database(&["libs/ships/first.xml", "libs/ships/moved.xml"]),
    )?;

    let changes = pair.compare_records(|comparison| {
        comparison
            .record_changes()
            .iter()
            .map(|change| {
                (
                    change.kind(),
                    change
                        .old_record()
                        .map(|side| side.record_path().to_owned()),
                    change
                        .new_record()
                        .map(|side| side.record_path().to_owned()),
                )
            })
            .collect::<Vec<_>>()
    })?;

    assert_eq!(
        changes,
        [
            (
                RecordChangeKind::Moved,
                Some("libs/ships/second".to_owned()),
                Some("libs/ships/moved".to_owned())
            ),
            (
                RecordChangeKind::Removed,
                Some("libs/ships/third".to_owned()),
                None
            ),
        ],
        "first with first, second with second, the third left over"
    );
    Ok(())
}
