//! The stderr notes of `p4kx loc` for a `global.ini` that was read lossily or had lines skipped.

use std::error::Error;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, GlobalIniBuilder, TestEntry};
use predicates::prelude::{PredicateBooleanExt, predicate};

type TestResult = Result<(), Box<dyn Error>>;

const ENGLISH: &str = "Data\\Localization\\english\\global.ini";
const GERMAN: &str = "Data\\Localization\\german_(germany)\\global.ini";

fn clean_english() -> TestEntry {
    TestEntry::zstd(
        ENGLISH,
        GlobalIniBuilder::new()
            .text("item_NameRifle", "Rifle")
            .build(),
    )
}

fn german_with_a_stray_line() -> TestEntry {
    TestEntry::zstd(
        GERMAN,
        b"item_NameRifle=Gewehr\r\nno equals line\r\n".to_vec(),
    )
}

fn archive(dir: &Path, entries: Vec<TestEntry>) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(Command::cargo_bin("p4kx")?)
}

#[test]
fn get_and_export_note_an_entry_with_skipped_lines() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        vec![clean_english(), german_with_a_stray_line()],
    )?;
    let note = format!("note: {GERMAN}: 1 line skipped\n");
    p4kx()?
        .args(["loc", "get"])
        .arg(&archive)
        .args(["item_NameRifle", "--lang", "de"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains(note.clone()))
        .stderr(predicate::str::contains("english").not());
    p4kx()?
        .args(["loc", "export"])
        .arg(&archive)
        .assert()
        .code(0)
        .stderr(predicate::str::contains(note));
    Ok(())
}

#[test]
fn get_notes_replaced_bytes_together_with_skipped_lines() -> TestResult {
    let dir = tempfile::tempdir()?;
    let damaged = TestEntry::zstd(
        ENGLISH,
        b"a=ok\r\nb=bad\xff\r\nnot a line\r\nalso not\r\n".to_vec(),
    );
    let archive = archive(dir.path(), vec![damaged])?;
    p4kx()?
        .args(["loc", "get"])
        .arg(&archive)
        .arg("a")
        .assert()
        .code(0)
        .stderr(predicate::str::contains(format!(
            "note: {ENGLISH}: invalid bytes replaced, 2 lines skipped\n"
        )));
    Ok(())
}

#[test]
fn a_clean_global_ini_prints_no_note() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path(), vec![clean_english()])?;
    p4kx()?
        .args(["loc", "get"])
        .arg(&archive)
        .arg("item_NameRifle")
        .assert()
        .code(0)
        .stderr("");
    Ok(())
}
