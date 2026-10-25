// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record name and Record tag in `p4kx dcb export`, for DataForge versions 6 and 8.

use std::error::Error;
use std::fs;

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

const TAGGED: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const UNTAGGED: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;

fn database(version: u32) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(version)
        .structure(TestStruct::new("Item").field("mass", TestType::Int32))
        .record(
            TestRecord::new(
                TAGGED,
                "Item",
                "Item.Box",
                "libs/foundry/records/items/shared.xml",
            )
            .tag("SystemsDesign")
            .value("mass", TestValue::Int32(1)),
        )
        .record(
            TestRecord::new(
                UNTAGGED,
                "Item",
                "Item.Crate",
                "libs/foundry/records/items/shared.xml",
            )
            .value("mass", TestValue::Int32(2)),
        )
}

fn exported(version: u32, args: &[&str], file: &str) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database(version).build()).encrypted())
        .write_to(&archive)?;
    let out = dir.path().join("out");
    let output = p4kx_testkit::p4kx()?
        .args(["dcb", "export"])
        .arg(&archive)
        .arg("--out")
        .arg(&out)
        .args(args)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "exit code: {output:?}");
    Ok(fs::read_to_string(out.join(file))?)
}

#[test]
fn xml_carries_the_name_and_a_version_8_tag() -> TestResult {
    insta::assert_snapshot!("xml_v8", exported(8, &["--combined"], "Game2.xml")?);
    insta::assert_snapshot!("xml_v6", exported(6, &["--combined"], "Game2.xml")?);
    Ok(())
}

#[test]
fn json_carries_the_name_and_a_version_8_tag() -> TestResult {
    let ndjson = ["--format", "ndjson"];
    insta::assert_snapshot!("ndjson_v8", exported(8, &ndjson, "records.ndjson")?);
    insta::assert_snapshot!("ndjson_v6", exported(6, &ndjson, "records.ndjson")?);
    Ok(())
}
