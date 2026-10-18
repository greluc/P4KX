//! `p4kx list` end to end.

use std::error::Error;

use assert_cmd::Command;
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

    let output = Command::cargo_bin("p4kx")?
        .arg("list")
        .arg(&archive)
        .output()?;

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
fn list_of_a_file_that_is_not_an_archive_fails_with_a_message() -> TestResult {
    let dir = tempfile::tempdir()?;
    let not_archive = dir.path().join("notes.txt");
    std::fs::write(&not_archive, "plain text")?;

    Command::cargo_bin("p4kx")?
        .arg("list")
        .arg(&not_archive)
        .assert()
        .failure()
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

    Command::cargo_bin("p4kx")?
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

    Command::cargo_bin("p4kx")?
        .arg("list")
        .arg(dir.path().join("missing.p4k"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read"));
    Ok(())
}
