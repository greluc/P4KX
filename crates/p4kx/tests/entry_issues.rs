//! `p4kx info`, `list` and `extract` on an Archive with defective Entries.

use std::error::Error;

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

fn write_archive(dir: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let archive = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", b"hello".to_vec()))
        .entry(TestEntry::stored("x", b"odd".to_vec()).with_path_bytes(b"Data\\b\xE9.txt"))
        .entry(TestEntry::stored("Data\\c.txt", b"three".to_vec()).with_encryption_field(2))
        .write_to(&archive)?;
    Ok(archive)
}

#[test]
fn info_counts_the_issues_and_warns_about_each_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("info")
        .arg(&archive)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Issues:"))
        .stderr(predicate::str::contains(
            "Entry Data\\b\u{FFFD}.txt: its path is not UTF-8",
        ))
        .stderr(predicate::str::contains(
            "Entry Data\\c.txt: its encryption field is 2",
        ));
    Ok(())
}

#[test]
fn info_json_has_the_issue_count() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("--json")
        .arg("info")
        .arg(&archive)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("\"issue_count\":2"));
    Ok(())
}
