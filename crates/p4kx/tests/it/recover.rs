// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `--recover` on the command line.

use std::error::Error;

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

const END: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];

fn damaged_archive(dir: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let mut bytes = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .entry(TestEntry::stored("Data\\b.xml", b"<b/>".to_vec()))
        .build();
    let end = bytes
        .windows(4)
        .rposition(|window| window == END)
        .ok_or("no end record")?;
    bytes.get_mut(end..end + 4).ok_or("range")?.fill(0);
    let path = dir.join("Data.p4k");
    std::fs::write(&path, bytes)?;
    Ok(path)
}

#[test]
fn a_damaged_archive_lists_only_with_recover() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = damaged_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("list")
        .arg(&archive)
        .assert()
        .failure();
    p4kx_testkit::p4kx()?
        .args(["--recover", "list"])
        .arg(&archive)
        .assert()
        .success()
        .stdout(predicate::str::contains("Data\\b.xml"))
        .stderr(predicate::str::contains("recovered"));
    Ok(())
}
