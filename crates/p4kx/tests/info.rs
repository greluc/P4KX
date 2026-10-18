//! `p4kx info` end to end.

use std::error::Error;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

fn write_channel(library: &Path, channel: &str) -> Result<PathBuf, Box<dyn Error>> {
    let folder = library.join(channel);
    std::fs::create_dir_all(&folder)?;
    let archive = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .entry(TestEntry::zstd("Data\\secret.xml", vec![b'x'; 1000]).encrypted())
        .write_to(&archive)?;
    Ok(archive)
}

#[test]
fn info_prints_the_channel_the_build_and_an_archive_summary() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_channel(dir.path(), "PTU")?;
    write_build_manifest(
        dir.path().join("PTU").as_path(),
        "sc-alpha-4.11.0",
        "4.11.42.1234",
        "12999999",
        "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0",
    )?;

    Command::cargo_bin("p4kx")?
        .arg("info")
        .arg(&archive)
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(r"(?m)^Channel: +PTU$")?)
        .stdout(predicate::str::is_match(
            r"(?m)^Branch: +sc-alpha-4\.11\.0$",
        )?)
        .stdout(predicate::str::is_match(
            r"(?m)^Version: +4\.11\.42\.1234$",
        )?)
        .stdout(predicate::str::is_match(r"(?m)^Change number: +12999999$")?)
        .stdout(predicate::str::is_match(
            r"(?m)^BuildId: +0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0$",
        )?)
        .stdout(predicate::str::is_match(r"(?m)^Entries: +2$")?)
        .stdout(predicate::str::is_match(r"(?m)^Encrypted: +1$")?)
        .stdout(predicate::str::is_match(r"(?m)^Size: +1005$")?);
    Ok(())
}

#[test]
fn info_of_an_archive_without_a_build_manifest_reports_an_unknown_build() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_channel(dir.path(), "LIVE")?;

    Command::cargo_bin("p4kx")?
        .arg("info")
        .arg(&archive)
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(r"(?m)^Channel: +LIVE$")?)
        .stdout(predicate::str::is_match(r"(?m)^Build: +unknown$")?)
        .stdout(predicate::str::is_match(r"(?m)^Entries: +2$")?);
    Ok(())
}

#[test]
fn info_with_a_malformed_build_manifest_warns_and_reports_an_unknown_build() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_channel(dir.path(), "LIVE")?;
    std::fs::write(
        dir.path().join("LIVE").join("build_manifest.id"),
        "{ broken",
    )?;

    Command::cargo_bin("p4kx")?
        .arg("info")
        .arg(&archive)
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(r"(?m)^Build: +unknown$")?)
        .stderr(predicate::str::contains("is not a valid build manifest"));
    Ok(())
}

#[test]
fn info_of_a_file_that_is_not_an_archive_exits_with_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let not_archive = dir.path().join("notes.txt");
    std::fs::write(&not_archive, "plain text")?;

    Command::cargo_bin("p4kx")?
        .arg("info")
        .arg(&not_archive)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("is not a p4k Archive"));
    Ok(())
}
