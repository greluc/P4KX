// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx list` end to end.

use std::error::Error;

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn list_prints_one_line_per_entry_with_method_encryption_and_sizes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\Config\\defaultProfile.xml",
            b"<Profile/>".to_vec(),
        ))
        .entry(TestEntry::stored("Data\\Scripts\\empty.lua", Vec::new()).flagged_encrypted())
        .write_to(&archive)?;

    let output = p4kx_testkit::p4kx()?.arg("list").arg(&archive).output()?;

    assert!(output.status.success(), "list succeeds: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .skip(1)
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(
        rows,
        vec![
            vec![
                "store",
                "no",
                "10",
                "10",
                "Data\\Libs\\Config\\defaultProfile.xml"
            ],
            vec!["store", "yes", "0", "0", "Data\\Scripts\\empty.lua"],
        ],
        "one row per Entry after the header: method, encrypted, compressed size, size, path"
    );
    Ok(())
}

#[test]
fn list_prints_the_entries_of_a_v2_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::v2()
        .entry(TestEntry::deflated("Data\\a.xml", b"<deflate/>".to_vec()))
        .entry(TestEntry::zstd("Data\\b.xml", b"<zstd/>".to_vec()).encrypted())
        .write_to(&archive)?;

    let output = p4kx_testkit::p4kx()?.arg("list").arg(&archive).output()?;

    assert!(output.status.success(), "list succeeds: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let rows: Vec<(&str, &str, &str)> = stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            Some((*columns.first()?, *columns.get(1)?, *columns.last()?))
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("deflate", "no", "Data\\a.xml"),
            ("zstd", "yes", "Data\\b.xml"),
        ],
        "one row per Entry of the v2 Archive"
    );
    Ok(())
}

#[test]
fn list_of_an_archive_in_an_unknown_layout_names_the_layout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    let mut bytes = ArchiveBuilder::v2()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .build();
    let version = bytes.len() - 6;
    bytes
        .get_mut(version..version + 2)
        .ok_or("no layout version")?
        .copy_from_slice(&7_u16.to_le_bytes());
    std::fs::write(&archive, bytes)?;

    p4kx_testkit::p4kx()?
        .arg("list")
        .arg(&archive)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("layout v7"));
    Ok(())
}

#[test]
fn list_of_a_file_that_is_not_an_archive_fails_with_a_message() -> TestResult {
    let dir = tempfile::tempdir()?;
    let not_archive = dir.path().join("notes.txt");
    std::fs::write(&not_archive, "plain text")?;

    p4kx_testkit::p4kx()?
        .arg("list")
        .arg(&not_archive)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("is not a p4k Archive"));
    Ok(())
}

#[test]
fn list_of_a_corrupt_archive_fails_with_the_offset() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    let mut bytes = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .build();
    let header = bytes
        .windows(4)
        .position(|window| window == [0x50, 0x4B, 0x01, 0x02])
        .ok_or("no central directory header in the synthetic Archive")?;
    bytes[header + 2] = 0x09;
    std::fs::write(&archive, bytes)?;

    p4kx_testkit::p4kx()?
        .arg("list")
        .arg(&archive)
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "is corrupt at offset {header}"
        )));
    Ok(())
}

#[test]
fn list_of_a_missing_file_fails_with_a_message() -> TestResult {
    let dir = tempfile::tempdir()?;

    p4kx_testkit::p4kx()?
        .arg("list")
        .arg(dir.path().join("missing.p4k"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read"));
    Ok(())
}
