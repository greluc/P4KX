// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The flat value export of References to Records that do not exist and of field names that
//! cannot stand in a field path.

use std::error::Error;

use p4kx_dataforge::{Annotations, DataForge, FlatFormat, FlatValues, TreeBuilder};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const ODD: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const GHOST: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;
const HEAD: &str = "cbee34fd-6441-4000-b99f-924f7f26e19f,Odd.One,libs/odd/one,";

fn flat(format: FlatFormat) -> Result<String, Box<dyn Error>> {
    let bytes = DataForgeBuilder::new()
        .structure(TestStruct::new("Base").field("x", TestType::Int32))
        .structure(
            TestStruct::new("Odd")
                .parent("Base")
                .field("x", TestType::Int32)
                .field("$odd", TestType::Int32)
                .field("y", TestType::Int32)
                .field("ghost", TestType::Reference)
                .field("none", TestType::Reference),
        )
        .record(
            TestRecord::new(ODD, "Odd", "Odd.One", "libs/odd/one.xml")
                .value("x", TestValue::Int32(1))
                .value("$odd", TestValue::Int32(2))
                .value("y", TestValue::Int32(3))
                .value("ghost", TestValue::Reference(GHOST))
                .value("none", TestValue::Null),
        )
        .build();
    let forge = DataForge::open(&bytes)?;
    let record = forge.records().next().ok_or("no Record")?;
    let tree = TreeBuilder::new().build(&record)?;
    let mut out = Vec::new();
    FlatValues::new(format).write_annotated(&tree, Annotations::none(), &mut out)?;
    Ok(String::from_utf8(out)?)
}

#[test]
fn a_reference_to_a_missing_record_is_a_reference_row_without_target_path() -> TestResult {
    let text = flat(FlatFormat::CsvRaw)?;

    assert!(
        text.contains(&format!(
            "{HEAD}ghost,,reference,00000001-0002-0003-0004-000000000005,\r\n"
        )),
        "{text}"
    );
    assert!(!text.contains(",none,"), "a null Reference has no row");
    Ok(())
}

#[test]
fn a_missing_record_is_a_reference_row_in_ndjson_too() -> TestResult {
    let text = flat(FlatFormat::Ndjson)?;

    let row = text
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|row| row["field_path"] == "ghost")
        .ok_or("ghost")?;
    assert_eq!(row["kind"], "reference");
    assert_eq!(row["value"], "00000001-0002-0003-0004-000000000005");
    assert!(row.get("target_path").is_none());
    Ok(())
}

#[test]
fn repeated_names_and_names_with_a_dollar_take_their_position_in_the_path() -> TestResult {
    let text = flat(FlatFormat::CsvRaw)?;
    let rows: Vec<&str> = text.split("\r\n").collect();

    assert!(
        rows.contains(&format!("{HEAD}x,,int,1,").as_str()),
        "{rows:#?}"
    );
    assert!(
        rows.contains(&format!("{HEAD}$1,,int,0,").as_str()),
        "{rows:#?}"
    );
    assert!(
        rows.contains(&format!("{HEAD}$2,,int,2,").as_str()),
        "{rows:#?}"
    );
    assert!(
        rows.contains(&format!("{HEAD}y,,int,3,").as_str()),
        "{rows:#?}"
    );
    Ok(())
}
