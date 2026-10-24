// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Build of an Archive, read from the Channel folder's `build_manifest.id`.

use std::error::Error;

use p4kx_archive::{Archive, Build};
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};

type TestResult = Result<(), Box<dyn Error>>;

const BUILD_ID: &str = "2cae0dfc-a941-4dcc-b82a-543b08b52d97";

fn write_archive(folder: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let archive = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .write_to(&archive)?;
    Ok(archive)
}

#[test]
fn an_archive_reads_its_build_from_the_manifest_next_to_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    write_build_manifest(
        dir.path(),
        "sc-alpha-4.10.2",
        "4.10.197.2878",
        "12913470",
        BUILD_ID,
    )?;

    let archive = Archive::open(archive)?;

    let build = archive.build().ok_or("the Build is known")?;
    assert_eq!(build.branch(), "sc-alpha-4.10.2", "branch");
    assert_eq!(build.version(), "4.10.197.2878", "version");
    assert_eq!(build.change_number(), "12913470", "change number");
    assert_eq!(build.build_id(), BUILD_ID, "BuildId");
    Ok(())
}

#[test]
fn an_archive_without_a_build_manifest_opens_with_an_unknown_build() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = Archive::open(write_archive(dir.path())?)?;

    assert!(archive.build().is_none(), "unknown Build");
    assert_eq!(archive.entries().len(), 1, "the Archive still opens");
    Ok(())
}

#[test]
fn an_archive_with_a_malformed_build_manifest_opens_with_an_unknown_build() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    std::fs::write(dir.path().join("build_manifest.id"), "{ not json")?;

    assert!(Archive::open(archive)?.build().is_none(), "unknown Build");
    Ok(())
}

#[test]
fn reading_a_folder_without_a_manifest_yields_no_build() -> TestResult {
    let dir = tempfile::tempdir()?;

    assert!(
        Build::read_from(dir.path())?.is_none(),
        "no manifest, no Build"
    );
    Ok(())
}

#[test]
fn reading_a_malformed_manifest_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("build_manifest.id"), r#"{"Data": {}}"#)?;

    let error = Build::read_from(dir.path()).err().ok_or("an error")?;
    assert!(
        error.to_string().contains("build_manifest.id"),
        "names the file: {error}"
    );
    Ok(())
}
