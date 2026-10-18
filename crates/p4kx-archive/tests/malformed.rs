//! Opening files that are not valid Archives through the public API.

use std::error::Error;
use std::path::Path;

use p4kx_archive::{Archive, ArchiveError};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn two_entry_archive() -> Vec<u8> {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .entry(TestEntry::stored("Data\\b.xml", b"<b/>".to_vec()))
        .build()
}

fn open_bytes(dir: &Path, bytes: &[u8]) -> Result<Result<Archive, ArchiveError>, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    std::fs::write(&path, bytes)?;
    Ok(Archive::open(&path))
}

#[test]
fn a_file_without_end_record_is_not_an_archive() -> TestResult {
    let dir = tempfile::tempdir()?;

    let result = open_bytes(dir.path(), b"this is a text file, not a p4k Archive")?;

    assert!(
        matches!(result, Err(ArchiveError::NotAnArchive { .. })),
        "a text file is rejected as not an Archive: {result:?}"
    );
    Ok(())
}

#[test]
fn an_empty_file_is_not_an_archive() -> TestResult {
    let dir = tempfile::tempdir()?;

    let result = open_bytes(dir.path(), &[])?;

    assert!(
        matches!(result, Err(ArchiveError::NotAnArchive { .. })),
        "an empty file is rejected as not an Archive: {result:?}"
    );
    Ok(())
}

#[test]
fn a_damaged_central_directory_header_is_reported_with_its_offset() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut bytes = two_entry_archive();
    let first_header = bytes
        .windows(4)
        .position(|window| window == [0x50, 0x4B, 0x01, 0x02])
        .ok_or("no central directory header in the synthetic Archive")?;
    bytes[first_header + 2] = 0x09;

    let result = open_bytes(dir.path(), &bytes)?;

    assert!(
        matches!(result, Err(ArchiveError::Corrupt { offset, .. }) if offset == first_header as u64),
        "the damaged header is reported at its file offset {first_header}: {result:?}"
    );
    Ok(())
}

#[test]
fn a_cut_off_archive_is_not_an_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = two_entry_archive();

    let result = open_bytes(dir.path(), &bytes[..bytes.len() - 40])?;

    assert!(
        matches!(result, Err(ArchiveError::NotAnArchive { .. })),
        "an Archive whose end records were cut off is rejected: {result:?}"
    );
    Ok(())
}
