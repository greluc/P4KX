// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Entry paths are written only when every part is a file name valid on Linux and Windows.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{
    Archive, EntryFailure, ExtractOptions, ExtractionError, ExtractionReport, NameRule, extract,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn run(dir: &Path, paths: &[String]) -> Result<ExtractionReport, Box<dyn Error>> {
    let archive_path = dir.join("Data.p4k");
    let builder = paths.iter().fold(ArchiveBuilder::new(), |builder, path| {
        builder.entry(TestEntry::stored(path.as_str(), b"data".to_vec()))
    });
    builder.write_to(&archive_path)?;
    let archive = Archive::open_with_key(&archive_path, Some(p4kx_testkit::test_key()))?;
    let target = dir.join("out");
    fs::create_dir(&target)?;
    let entries: Vec<_> = archive.entries().iter().collect();
    Ok(extract(
        &archive,
        &entries,
        &target,
        &ExtractOptions::default(),
    ))
}

fn written(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir.join("out").join("Data"))? {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

fn paths(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| format!("Data\\{name}")).collect()
}

#[test]
fn an_entry_with_a_character_windows_forbids_is_reported_with_the_character() -> TestResult {
    let dir = tempfile::tempdir()?;
    let cases = [
        ("a<b.txt", NameRule::ForbiddenCharacter('<')),
        ("a>b.txt", NameRule::ForbiddenCharacter('>')),
        ("a\"b.txt", NameRule::ForbiddenCharacter('"')),
        ("a|b.txt", NameRule::ForbiddenCharacter('|')),
        ("a?b.txt", NameRule::ForbiddenCharacter('?')),
        ("a*b.txt", NameRule::ForbiddenCharacter('*')),
        ("a\u{1}b.txt", NameRule::ForbiddenCharacter('\u{1}')),
        ("a\u{1F}b.txt", NameRule::ForbiddenCharacter('\u{1F}')),
        ("trailing.", NameRule::TrailingDotOrSpace),
        ("con.txt", NameRule::DeviceName),
    ];
    let mut names: Vec<&str> = cases.iter().map(|(name, _)| *name).collect();
    names.push("fine.txt");

    let report = run(dir.path(), &paths(&names))?;

    assert_eq!(written(dir.path())?, ["fine.txt"], "only the valid name");
    assert_eq!(report.extracted(), 1, "the run goes on");
    assert_eq!(report.failures().len(), cases.len());
    for ((name, expected), failure) in cases.iter().zip(report.failures()) {
        assert_eq!(failure.path(), format!("Data\\{name}"), "names the Entry");
        assert!(
            matches!(
                failure.error(),
                ExtractionError::InvalidName { part, rule } if part == name && rule == expected
            ),
            "{name:?}: {:?}",
            failure.error()
        );
    }
    Ok(())
}

#[test]
fn a_name_over_255_bytes_is_reported_by_its_utf8_length() -> TestResult {
    let dir = tempfile::tempdir()?;
    let too_long = format!("{}.txt", "ä".repeat(126));

    let report = run(dir.path(), &paths(&[&too_long]))?;

    assert_eq!(report.extracted(), 0, "nothing is written");
    assert!(
        matches!(
            report.failures().first().map(EntryFailure::error),
            Some(ExtractionError::InvalidName {
                rule: NameRule::TooManyBytes(256),
                ..
            })
        ),
        "{report:?}"
    );
    Ok(())
}

#[test]
fn entries_that_differ_only_in_case_are_a_collision_on_every_platform() -> TestResult {
    let dir = tempfile::tempdir()?;

    let report = run(
        dir.path(),
        &paths(&["Same.txt", "other.txt", "same.TXT", "SAME.txt"]),
    )?;

    assert_eq!(written(dir.path())?, ["Same.txt", "other.txt"]);
    assert_eq!(report.failures().len(), 2, "both later Entries fail");
    for failure in report.failures() {
        assert!(
            matches!(
                failure.error(),
                ExtractionError::CaseCollision { earlier } if earlier == "Data\\Same.txt"
            ),
            "{failure:?}"
        );
    }
    Ok(())
}

#[test]
fn a_full_path_over_the_windows_limit_is_reported_for_its_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let deep = format!("Data\\{}\\{}\\x.txt", "d".repeat(120), "e".repeat(120));

    let report = run(dir.path(), &[deep.clone(), "Data\\short.txt".to_owned()])?;

    assert_eq!(report.extracted(), 1, "the short path is written");
    let failure = report.failures().first().ok_or("one failure")?;
    assert_eq!(failure.path(), deep);
    assert!(
        matches!(failure.error(), ExtractionError::PathTooLong(_)),
        "{failure:?}"
    );
    Ok(())
}
