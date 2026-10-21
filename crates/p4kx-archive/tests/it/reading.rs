//! Reading the data of Entries through the public API.

use std::error::Error;
use std::io::Read;

use p4kx_archive::{Archive, ArchiveError};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn read_all(archive: &Archive, index: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive.entries().get(index).ok_or("Entry missing")?;
    let mut data = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut data)?;
    Ok(data)
}

#[test]
fn reads_the_data_of_stored_entries() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .entry(TestEntry::stored("Data\\empty.txt", Vec::new()))
        .entry(TestEntry::stored("Data\\b.xml", b"<b>second</b>".to_vec()))
        .write_to(&path)?;

    let archive = Archive::open(&path)?;

    assert_eq!(read_all(&archive, 0)?, b"<a/>", "first Entry data");
    assert_eq!(read_all(&archive, 1)?, b"", "empty Entry data");
    assert_eq!(read_all(&archive, 2)?, b"<b>second</b>", "last Entry data");
    Ok(())
}

#[test]
fn a_broken_local_header_is_reported_as_corrupt() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let mut bytes = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .build();
    bytes[0] = 0x00;
    std::fs::write(&path, bytes)?;
    let archive = Archive::open(&path)?;
    let entry = archive.entries().first().ok_or("Entry missing")?;

    let result = archive.open_entry(entry);

    assert!(
        matches!(result, Err(ArchiveError::Corrupt { offset: 0, .. })),
        "the local header at offset 0 is corrupt: {result:?}"
    );
    Ok(())
}
