// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The test-build notice of the command line: one line on stderr for every Channel but `LIVE`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry};
use predicates::prelude::{PredicateBooleanExt, predicate};

type TestResult = Result<(), Box<dyn Error>>;

fn write_channel(library: &Path, channel: &str) -> Result<PathBuf, Box<dyn Error>> {
    let folder = library.join(channel);
    std::fs::create_dir_all(&folder)?;
    let archive = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .entry(TestEntry::stored(
            "Data\\Game2.dcb",
            DataForgeBuilder::new().build(),
        ))
        .write_to(&archive)?;
    Ok(archive)
}

#[test]
fn every_test_channel_gets_one_notice_line_on_stderr() -> TestResult {
    let dir = tempfile::tempdir()?;
    for channel in ["PTU", "EPTU", "HOTFIX", "TECH-PREVIEW"] {
        let archive = write_channel(dir.path(), channel)?;
        let assert = p4kx_testkit::p4kx()?
            .arg("list")
            .arg(&archive)
            .assert()
            .code(0)
            .stdout(predicate::str::contains("notice").not())
            .stderr(predicate::str::starts_with("notice: "))
            .stderr(predicate::str::contains(channel))
            .stderr(predicate::str::contains("confidential"));
        let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
        assert_eq!(stderr.lines().count(), 1, "{channel}: {stderr}");
    }
    Ok(())
}

#[test]
fn live_and_unknown_folders_get_no_notice() -> TestResult {
    let dir = tempfile::tempdir()?;
    for channel in ["LIVE", "Elsewhere"] {
        let archive = write_channel(dir.path(), channel)?;
        p4kx_testkit::p4kx()?
            .arg("list")
            .arg(&archive)
            .assert()
            .code(0)
            .stderr(predicate::str::is_empty());
    }
    Ok(())
}

#[test]
fn json_output_stays_clean_and_the_notice_stays_on_stderr() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_channel(dir.path(), "PTU")?;
    let assert = p4kx_testkit::p4kx()?
        .arg("--json")
        .arg("info")
        .arg(&archive)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("confidential"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["channel"], "PTU");
    assert!(!stdout.contains("confidential"), "{stdout}");
    Ok(())
}

#[test]
fn comparing_a_test_channel_with_live_still_works_and_notes_it_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let live = write_channel(dir.path(), "LIVE")?;
    let ptu = write_channel(dir.path(), "PTU")?;
    let assert = p4kx_testkit::p4kx()?
        .arg("compare")
        .arg(&live)
        .arg(&ptu)
        .assert()
        .code(0);
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    assert_eq!(stderr.matches("notice: ").count(), 1, "{stderr}");
    Ok(())
}
