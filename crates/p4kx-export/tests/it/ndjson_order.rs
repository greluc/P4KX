// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The record-path order of the NDJSON Record export.

use std::error::Error;
use std::fs;

use p4kx_export::{NDJSON_FILE, RecordExportOptions, RecordForm, write_record_export};
use p4kx_extraction::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

fn ndjson_numbers(builder: &DataForgeBuilder) -> Result<Vec<u64>, Box<dyn Error>> {
    let bytes = builder.build();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();
    let dir = tempfile::tempdir()?;
    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Ndjson,
        dir.path(),
        &RecordExportOptions::default(),
    );
    assert!(summary.is_complete(), "{:?}", summary.failures());
    let text = fs::read_to_string(dir.path().join(NDJSON_FILE))?;
    let numbers = text
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).map(|value| {
                value
                    .get("n")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(u64::MAX)
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(numbers)
}

fn items(paths: &[(&str, &str)]) -> DataForgeBuilder {
    (0_u32..).zip(paths).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Item").field("n", TestType::UInt32)),
        |builder, (n, (name, path))| {
            builder.record(
                TestRecord::new(u128::from(n) + 1, "Item", *name, *path)
                    .value("n", TestValue::UInt32(n)),
            )
        },
    )
}

#[test]
fn record_paths_compare_ignoring_ascii_case_and_separators_then_names() -> TestResult {
    let builder = items(&[
        ("Item.B", "libs/Zeta.xml"),
        ("Item.A", "libs/alpha.xml"),
        ("Item.C", "LIBS\\beta.xml"),
        ("Item.B", "libs/Alpha.xml"),
        ("Item.A", "libs/zeta.xml"),
    ]);

    let numbers = ndjson_numbers(&builder)?;

    assert_eq!(
        numbers,
        [1, 3, 2, 4, 0],
        "alpha (A, then B), beta, zeta (A, then B)"
    );
    Ok(())
}
#[test]
fn ndjson_keeps_record_path_order_across_batches() -> TestResult {
    let count = 5_000_u32;
    let builder = (0..count).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Item").field("n", TestType::UInt32)),
        |builder, n| {
            builder.record(
                TestRecord::new(
                    u128::from(n) + 1,
                    "Item",
                    format!("Item.{n}"),
                    format!("items/{:05}.xml", count - n),
                )
                .value("n", TestValue::UInt32(n)),
            )
        },
    );
    let numbers = ndjson_numbers(&builder)?;
    let expected: Vec<u64> = (0..u64::from(count)).rev().collect();
    assert_eq!(
        numbers, expected,
        "sorted by record path, not by definition"
    );
    Ok(())
}
