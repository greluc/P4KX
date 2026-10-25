// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx search` and `p4kx extract --filter` end to end.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let archive = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\Config\\defaultProfile.xml",
            b"<Profile/>".to_vec(),
        ))
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .entry(TestEntry::zstd("Data\\Libs\\secret.xml", b"<secret/>".to_vec()).encrypted())
        .write_to(&archive)?;
    Ok(archive)
}

fn searched_paths(archive: &Path, filter: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let output = p4kx_testkit::p4kx()?
        .arg("search")
        .arg(archive)
        .arg(filter)
        .output()?;
    assert!(output.status.success(), "search succeeds: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let mut lines = stdout.lines();
    let header = lines.next().unwrap_or_default();
    assert!(header.starts_with("METHOD"), "list header: {header}");
    Ok(lines
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect())
}

#[test]
fn search_lists_the_entries_containing_the_text_ignoring_case() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    assert_eq!(
        searched_paths(&archive, "data/LIBS")?,
        vec![
            "Data\\Libs\\Config\\defaultProfile.xml",
            "Data\\Libs\\secret.xml"
        ],
        "substring with either separator and any case"
    );
    Ok(())
}

#[test]
fn search_with_a_glob_matches_the_full_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    assert_eq!(
        searched_paths(&archive, "*.xml")?,
        vec![
            "Data\\Libs\\Config\\defaultProfile.xml",
            "Data\\Libs\\secret.xml"
        ],
        "star crosses separators"
    );
    assert!(
        searched_paths(&archive, "Libs*")?.is_empty(),
        "a glob is anchored at the start"
    );
    Ok(())
}

#[test]
fn search_without_a_match_prints_only_the_header_and_succeeds() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    assert!(searched_paths(&archive, "missing")?.is_empty(), "no rows");
    Ok(())
}

#[test]
fn search_of_a_file_that_is_not_an_archive_exits_with_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let not_archive = dir.path().join("notes.txt");
    std::fs::write(&not_archive, "plain text")?;

    p4kx_testkit::p4kx()?
        .arg("search")
        .arg(&not_archive)
        .arg("x")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("is not a p4k Archive"));
    Ok(())
}

#[test]
fn extract_with_a_filter_writes_every_matching_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("--filter")
        .arg("*.XML")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 2, skipped 0, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read(out.join("Data/Libs/secret.xml"))?,
        b"<secret/>",
        "encrypted match"
    );
    assert!(
        out.join("Data/Libs/Config/defaultProfile.xml").exists(),
        "nested match"
    );
    assert!(!out.join("Data/readme.txt").exists(), "no match, no file");
    Ok(())
}

#[test]
fn extract_with_a_filter_that_matches_nothing_exits_with_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("--filter")
        .arg("missing")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no Entry matches missing"))
        .stderr(predicate::str::contains(
            "hint: a Filter without * or ? matches text anywhere in the path",
        ));

    assert!(!out.exists(), "no target is created for invalid input");
    Ok(())
}

#[test]
fn a_glob_that_matches_nothing_hints_at_the_whole_path_rule() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("--filter")
        .arg("Config/*.xml")
        .arg("--out")
        .arg(dir.path().join("out"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "hint: a glob must match the whole path",
        ));
    Ok(())
}

#[test]
fn a_filter_with_surrounding_whitespace_hints_at_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("--filter")
        .arg("readme ")
        .arg("--out")
        .arg(dir.path().join("out"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("leading or trailing whitespace"));
    Ok(())
}

#[test]
fn extract_with_threads_writes_the_same_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .args(["--filter", "*.XML", "--threads", "2", "--out"])
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 2, skipped 0, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read(out.join("Data/Libs/secret.xml"))?,
        b"<secret/>",
        "encrypted match"
    );
    Ok(())
}

#[test]
fn extract_rejects_zero_threads() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .args(["--filter", "*.XML", "--threads", "0"])
        .assert()
        .code(2);
    Ok(())
}

#[test]
fn extract_takes_either_entry_paths_or_a_filter() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .arg("--filter")
        .arg("*.xml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
    Ok(())
}
