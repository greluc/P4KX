// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Record names and record paths are written only as file names valid on Linux and Windows.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{
    DataForge, NameRule, RecordExportOptions, RecordExportReport, RecordRenamed, RenameCause,
    export_records,
};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID_1: &str = "00000000-0000-0000-0000-000000000001";

fn item(guid: u128, name: &str, file: &str) -> TestRecord {
    TestRecord::new(guid, "Item", name, file).value("count", TestValue::Int32(7))
}

fn export(
    records: impl IntoIterator<Item = TestRecord>,
    target: &Path,
) -> Result<RecordExportReport, Box<dyn Error>> {
    let bytes = records
        .into_iter()
        .fold(
            DataForgeBuilder::new()
                .structure(TestStruct::new("Item").field("count", TestType::Int32)),
            DataForgeBuilder::record,
        )
        .build();
    let forge = DataForge::open(&bytes)?;
    let selected: Vec<_> = forge.records().collect();
    Ok(export_records(
        &forge,
        &selected,
        target,
        &RecordExportOptions::default(),
    ))
}

fn files(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let relative = path.strip_prefix(root)?.display().to_string();
                found.push(relative.replace('\\', "/"));
            }
        }
    }
    found.sort();
    Ok(found)
}

#[test]
fn a_record_name_with_a_forbidden_character_is_written_under_its_guid() -> TestResult {
    let dir = tempfile::tempdir()?;

    let report = export(
        [
            item(1, "Item.a?b", "records/shared.xml"),
            item(2, "Item.fine", "records/shared.xml"),
        ],
        dir.path(),
    )?;

    assert!(report.is_complete(), "{report:?}");
    assert_eq!(
        files(dir.path())?,
        [
            format!("records/shared/{GUID_1}.xml"),
            "records/shared/Item.fine.xml".to_owned()
        ],
        "the Record that could be lost is kept under its GUID"
    );
    let [renamed] = report.renamed() else {
        return Err(format!("one note: {:?}", report.renamed()).into());
    };
    assert_eq!(
        renamed.cause(),
        RenameCause::Name(NameRule::ForbiddenCharacter('?'))
    );
    assert_eq!(renamed.guid().to_string(), GUID_1);
    Ok(())
}

#[test]
fn a_record_path_part_with_a_forbidden_character_is_escaped_without_collisions() -> TestResult {
    let dir = tempfile::tempdir()?;

    let report = export(
        [
            item(1, "Item.pipe", "records/a|b.xml"),
            item(2, "Item.literal", "records/a%7Cb.xml"),
            item(3, "Item.star", "records/s*.xml"),
            item(4, "Item.dot", "records/dot./x.xml"),
        ],
        dir.path(),
    )?;

    assert!(report.is_complete(), "{report:?}");
    assert_eq!(
        files(dir.path())?,
        [
            "records/a%257Cb.xml",
            "records/a%7Cb.xml",
            "records/dot%2E/x.xml",
            "records/s%2A.xml",
        ],
        "two different paths never share a file"
    );
    let causes: Vec<_> = report.renamed().iter().map(RecordRenamed::cause).collect();
    assert_eq!(
        causes,
        [
            RenameCause::Path(NameRule::ForbiddenCharacter('|')),
            RenameCause::Path(NameRule::ForbiddenCharacter('*')),
            RenameCause::Path(NameRule::TrailingDotOrSpace),
        ],
        "the `%` alone is no rule broken"
    );
    Ok(())
}

#[test]
fn a_record_name_that_looks_like_an_escape_gives_way_to_the_escaped_path() -> TestResult {
    let dir = tempfile::tempdir()?;

    let report = export(
        [
            item(1, "%3C", "x/shared.xml"),
            item(2, "Item.fine", "x/shared.xml"),
            item(3, "Item.escaped", "x/shared/<.xml"),
        ],
        dir.path(),
    )?;

    assert!(report.is_complete(), "{report:?}");
    assert_eq!(
        files(dir.path())?,
        [
            format!("x/shared/%3C.{GUID_1}.xml"),
            "x/shared/%3C.xml".to_owned(),
            "x/shared/Item.fine.xml".to_owned(),
        ],
        "the Record alone at its path keeps the file; the shared Record adds its GUID"
    );
    Ok(())
}

#[test]
fn a_record_path_part_no_escape_can_fit_goes_into_the_long_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let long = format!("records/{}.xml", "<".repeat(100));

    let report = export(
        [
            item(1, "Item.long", &long),
            item(2, "Item.fine", "records/fine.xml"),
        ],
        dir.path(),
    )?;

    assert!(report.is_complete(), "{report:?}");
    assert_eq!(
        files(dir.path())?,
        [format!("%long/{GUID_1}.xml"), "records/fine.xml".to_owned()]
    );
    Ok(())
}

#[test]
fn a_file_name_over_the_limit_after_the_extension_goes_into_the_long_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let name = "n".repeat(252);

    let report = export(
        [
            item(1, "Item.a", &format!("records/{name}.xml")),
            item(2, "Item.b", "records/b.xml"),
        ],
        dir.path(),
    )?;

    assert!(report.is_complete(), "{report:?}");
    assert_eq!(
        files(dir.path())?,
        [format!("%long/{GUID_1}.xml"), "records/b.xml".to_owned()],
        "252 characters and `.xml` are 256 bytes"
    );
    Ok(())
}
