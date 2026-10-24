// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Record layouts the Record tree rejects.

use std::error::Error;

use p4kx_dataforge::{DataForge, DataForgeError, ValueError};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
#[test]
fn error_values_carry_a_reason() {
    assert_eq!(
        ValueError::InvalidString {
            table: "text",
            offset: 7
        }
        .to_string(),
        "offset 7 in the text table is not a valid string",
        "string reason"
    );
    assert_eq!(
        ValueError::UnknownEnum { index: 3 }.to_string(),
        "the field refers to enum 3, which does not exist",
        "enum reason"
    );
    assert_eq!(
        ValueError::InvalidBool { byte: 2 }.to_string(),
        "the bool byte 2 is neither 0 nor 1",
        "bool reason"
    );
}

#[test]
fn a_cycle_in_the_struct_parents_is_an_error() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(TestStruct::new("A").parent("B").field("a", TestType::Int8))
        .structure(TestStruct::new("B").parent("A").field("b", TestType::Int8))
        .record(TestRecord::new(SAMPLE, "A", "A.One", "a.xml"))
        .build();
    let forge = DataForge::open(&bytes)?;
    let record = forge.records().next().ok_or("no Record")?;

    let result = record.tree();

    assert!(
        matches!(result, Err(DataForgeError::StructCycle { struct_index: 0 })),
        "the cycle is rejected: {result:?}"
    );
    Ok(())
}

#[test]
fn a_struct_whose_fields_do_not_fill_its_size_is_an_error() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Odd")
                .field("a", TestType::Int32)
                .declared_size(6),
        )
        .record(TestRecord::new(SAMPLE, "Odd", "Odd.One", "a.xml"))
        .build();
    let forge = DataForge::open(&bytes)?;
    let record = forge.records().next().ok_or("no Record")?;

    let result = record.tree();

    assert!(
        matches!(
            result,
            Err(DataForgeError::StructLayout {
                struct_index: 0,
                declared: 6,
                fields: 4
            })
        ),
        "the layout is rejected: {result:?}"
    );
    Ok(())
}

#[test]
fn an_inline_class_of_an_unknown_struct_is_an_error() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Holder")
                .field("a", TestType::Int32)
                .field("inner", TestType::Class("Missing".to_owned())),
        )
        .record(TestRecord::new(SAMPLE, "Holder", "Holder.One", "a.xml"))
        .build();
    let forge = DataForge::open(&bytes)?;
    let record = forge.records().next().ok_or("no Record")?;

    let result = record.tree();

    assert!(
        matches!(
            result,
            Err(DataForgeError::IndexOutOfRange {
                part: "property",
                target: "struct",
                target_index: 65_535,
                ..
            })
        ),
        "the target is range-checked: {result:?}"
    );
    Ok(())
}
