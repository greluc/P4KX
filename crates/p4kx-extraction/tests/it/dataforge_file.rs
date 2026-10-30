// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading a DataForge database from a file on disk.

use std::error::Error;
use std::fs::{self, File};
use std::io::Write;

use p4kx_extraction::{
    DataForgeEntry, DataForgeLoadError, LoadedDataForge, MAX_DATAFORGE_SIZE, is_dataforge_file,
};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};

type TestResult = Result<(), Box<dyn Error>>;

fn database(records: u128) -> Vec<u8> {
    (0..records)
        .fold(
            DataForgeBuilder::new().structure(TestStruct::new("Item")),
            |builder, guid| {
                builder.record(TestRecord::new(
                    guid + 1,
                    "Item",
                    format!("Item.N{guid}"),
                    "libs/foundry/records/items.xml",
                ))
            },
        )
        .build()
}

#[test]
fn a_file_is_read_by_content_whatever_its_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("kept-from-last-patch.bin");
    fs::write(&file, database(3))?;

    assert!(is_dataforge_file(&file)?, "the header is recognised");
    let entry = DataForgeEntry::read_file(&file)?;

    assert_eq!(entry.path(), "kept-from-last-patch.bin", "the file name");
    assert_eq!(entry.open()?.record_count(), 3, "its Records are read");
    let loaded = LoadedDataForge::load_file(&file)?;
    assert_eq!(loaded.with(|forge| forge.record_count())?, 3);
    Ok(())
}

#[test]
fn an_archive_and_other_files_are_not_dataforge_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Game2.dcb", database(1)))
        .write_to(&archive)?;
    let text = dir.path().join("readme.txt");
    fs::write(&text, "a text file that is long enough")?;
    let short = dir.path().join("short.bin");
    fs::write(&short, [0_u8; 4])?;

    assert!(!is_dataforge_file(&archive)?, "an Archive");
    assert!(!is_dataforge_file(&text)?, "a text");
    assert!(!is_dataforge_file(&short)?, "a file shorter than the header word pair");
    assert!(
        is_dataforge_file(&dir.path().join("missing.dcb")).is_err(),
        "a missing file cannot be looked at"
    );
    Ok(())
}

#[test]
fn an_unsupported_version_is_still_recognised() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("old.dcb");
    fs::write(&file, DataForgeBuilder::new().version(5).build())?;

    assert!(is_dataforge_file(&file)?, "version 5 has a DataForge header");
    let entry = DataForgeEntry::read_file(&file)?;
    assert!(entry.open().is_err(), "but it cannot be opened");
    Ok(())
}

#[test]
fn a_truncated_file_does_not_open() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("cut.dcb");
    let whole = database(4);
    fs::write(&file, &whole[..whole.len() / 2])?;

    let entry = DataForgeEntry::read_file(&file)?;

    assert!(entry.open().is_err(), "the tables end too early");
    assert!(matches!(
        LoadedDataForge::load_file(&file),
        Err(DataForgeLoadError::Corrupt(_))
    ));
    Ok(())
}

#[test]
fn a_file_over_the_limit_is_refused_before_it_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("huge.dcb");
    let mut handle = File::create(&file)?;
    handle.write_all(&database(1))?;
    handle.set_len(MAX_DATAFORGE_SIZE + 1)?;
    drop(handle);

    let Err(DataForgeLoadError::TooLarge { path, size, limit }) = DataForgeEntry::read_file(&file)
    else {
        return Err("a file over the limit is refused".into());
    };

    assert_eq!(path, "huge.dcb");
    assert_eq!(size, MAX_DATAFORGE_SIZE + 1);
    assert_eq!(limit, MAX_DATAFORGE_SIZE);
    Ok(())
}

#[test]
fn a_missing_file_is_a_read_error() {
    let dir = tempfile::tempdir().map(|dir| dir.path().join("missing.dcb"));
    let Ok(file) = dir else { return };

    assert!(matches!(
        DataForgeEntry::read_file(&file),
        Err(DataForgeLoadError::Read { .. })
    ));
}

#[test]
fn reading_stops_once_asked_to() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("Game2.dcb");
    fs::write(&file, database(1))?;

    assert!(matches!(
        DataForgeEntry::read_file_until(&file, &|| true),
        Err(DataForgeLoadError::Cancelled)
    ));
    Ok(())
}
