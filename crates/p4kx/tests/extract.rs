//! `p4kx extract` end to end.

use std::error::Error;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
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
        .entry(TestEntry::stored("..\\..\\escaped.txt", b"evil".to_vec()))
        .entry(TestEntry::stored("C:\\drive.txt", b"evil".to_vec()))
        .entry(TestEntry::stored("Data\\secret.xml", b"<s/>".to_vec()).flagged_encrypted())
        .write_to(&archive)?;
    Ok(archive)
}

fn export_folders(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = Vec::new();
    for item in std::fs::read_dir(dir)? {
        let name = item?.file_name().to_string_lossy().into_owned();
        if name.starts_with("SC-EXPORT-") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

#[test]
fn extract_writes_the_requested_entries_into_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");

    Command::cargo_bin("p4kx")?
        .arg("extract")
        .arg(&archive)
        .arg("Data/Libs/Config/defaultProfile.xml")
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 2, skipped 0, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read(out.join("Data/Libs/Config/defaultProfile.xml"))?,
        b"<Profile/>",
        "nested Entry content"
    );
    assert_eq!(
        std::fs::read(out.join("Data/readme.txt"))?,
        b"hello",
        "Entry content"
    );
    Ok(())
}

#[test]
fn extract_without_out_creates_a_new_default_target_for_every_run() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    for _ in 0..2 {
        Command::cargo_bin("p4kx")?
            .current_dir(dir.path())
            .arg("extract")
            .arg(&archive)
            .arg("Data\\readme.txt")
            .assert()
            .code(0);
    }

    let folders = export_folders(dir.path())?;
    assert_eq!(folders.len(), 2, "two runs, two folders: {folders:?}");
    for folder in &folders {
        assert!(
            folder.starts_with("SC-EXPORT-UnknownChannel-UnknownBuild-"),
            "default name pattern: {folder}"
        );
        assert_eq!(
            std::fs::read(dir.path().join(folder).join("Data/readme.txt"))?,
            b"hello",
            "each run extracts into its own folder"
        );
    }
    Ok(())
}

#[test]
fn extract_rejects_entry_paths_outside_the_target_with_exit_code_1() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("a").join("b").join("out");

    Command::cargo_bin("p4kx")?
        .arg("extract")
        .arg(&archive)
        .arg("..\\..\\escaped.txt")
        .arg("C:\\drive.txt")
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "extracted 1, skipped 0, failed 2, cancelled 0",
        ))
        .stderr(predicate::str::contains(
            "failed ..\\..\\escaped.txt: the path would leave the target folder",
        ))
        .stderr(predicate::str::contains(
            "failed C:\\drive.txt: the path would leave the target folder",
        ));

    assert!(
        !dir.path().join("a").join("escaped.txt").exists()
            && !dir.path().join("escaped.txt").exists(),
        "nothing is written outside the target"
    );
    assert_eq!(
        std::fs::read_dir(&out)?.count(),
        1,
        "only the Data folder is in the target"
    );
    Ok(())
}

#[test]
fn extract_of_an_unsupported_entry_fails_it_with_exit_code_1() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    Command::cargo_bin("p4kx")?
        .arg("extract")
        .arg(&archive)
        .arg("Data\\secret.xml")
        .arg("--out")
        .arg(dir.path().join("out"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("is encrypted"));
    Ok(())
}

#[test]
fn extract_keeps_existing_files_and_reports_them_as_skipped() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");
    std::fs::create_dir_all(out.join("Data"))?;
    std::fs::write(out.join("Data/readme.txt"), "mine")?;

    Command::cargo_bin("p4kx")?
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "skipped Data\\readme.txt: already exists",
        ))
        .stdout(predicate::str::contains(
            "extracted 0, skipped 1, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read_to_string(out.join("Data/readme.txt"))?,
        "mine",
        "the existing file is unchanged"
    );
    Ok(())
}

#[test]
fn extract_of_an_unknown_entry_is_invalid_input_with_exit_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    Command::cargo_bin("p4kx")?
        .current_dir(dir.path())
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .arg("Data\\missing.xml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no Entry named Data\\missing.xml"));

    assert!(
        export_folders(dir.path())?.is_empty(),
        "no target is created for invalid input"
    );
    Ok(())
}

#[test]
fn extract_without_entry_paths_is_invalid_input_with_exit_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    Command::cargo_bin("p4kx")?
        .arg("extract")
        .arg(&archive)
        .assert()
        .code(2);
    Ok(())
}

#[test]
fn extract_from_a_file_that_is_not_an_archive_exits_with_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let not_archive = dir.path().join("notes.txt");
    std::fs::write(&not_archive, "plain text")?;

    Command::cargo_bin("p4kx")?
        .arg("extract")
        .arg(&not_archive)
        .arg("Data\\readme.txt")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("is not a p4k Archive"));
    Ok(())
}

#[test]
fn extract_help_documents_the_exit_codes() -> TestResult {
    Command::cargo_bin("p4kx")?
        .args(["extract", "--help"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Exit codes:"))
        .stdout(predicate::str::contains("2  invalid input"))
        .stdout(predicate::str::contains("3  the Archive cannot be read"));
    Ok(())
}
