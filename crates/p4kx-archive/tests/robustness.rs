//! Archives with malformed fields that still open, and the defects reported for their Entries.

use std::error::Error;
use std::io::Read;
use std::path::Path;

use p4kx_archive::{Archive, ArchiveError, Entry, EntryIssue};
use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const END_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
const ZIP64_END_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x06, 0x06];
const V2_TRAILER_LEN: usize = 175;

fn open_bytes(dir: &Path, bytes: &[u8]) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    std::fs::write(&path, bytes)?;
    Ok(Archive::open(&path)?)
}

fn read_all(archive: &Archive, index: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive.entries().get(index).ok_or("Entry missing")?;
    let mut data = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut data)?;
    Ok(data)
}

fn v1(entry: TestEntry) -> Vec<u8> {
    ArchiveBuilder::new().entry(entry).build()
}

fn v2(entry: TestEntry) -> Vec<u8> {
    ArchiveBuilder::v2().entry(entry).build()
}

fn put_u64(bytes: &mut [u8], at: usize, value: u64) -> TestResult {
    bytes
        .get_mut(at..at + 8)
        .ok_or("field outside the synthetic Archive")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn last_position(bytes: &[u8], needle: &[u8]) -> Result<usize, Box<dyn Error>> {
    bytes
        .windows(needle.len())
        .rposition(|window| window == needle)
        .ok_or_else(|| "signature missing".into())
}

#[test]
fn a_clean_archive_reports_no_issues() -> TestResult {
    let dir = tempfile::tempdir()?;
    for bytes in [
        v1(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec())),
        v2(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec())),
    ] {
        let archive = open_bytes(dir.path(), &bytes)?;
        assert!(archive.issues().is_empty(), "{:?}", archive.issues());
    }
    Ok(())
}

#[test]
fn a_v1_encryption_block_value_of_zero_or_one_is_plain_or_encrypted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let plain = TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_field(0);
    let encrypted = TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_field(1);

    let plain = open_bytes(dir.path(), &v1(plain))?;
    let encrypted = open_bytes(dir.path(), &v1(encrypted))?;

    assert!(plain.issues().is_empty() && !plain.entries()[0].is_encrypted());
    assert!(encrypted.issues().is_empty() && encrypted.entries()[0].is_encrypted());
    Ok(())
}

#[test]
fn a_v1_encryption_block_value_above_one_is_reported_and_read_as_encrypted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let entry = TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_field(2);

    let archive = open_bytes(dir.path(), &v1(entry))?;

    assert_eq!(
        archive.issues(),
        [(0, EntryIssue::UnknownEncryption { value: 2 })]
    );
    assert!(archive.entries()[0].is_encrypted(), "treated as encrypted");
    Ok(())
}

#[test]
fn a_v1_encryption_block_of_another_size_is_reported_and_its_value_still_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let longer =
        TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_block([0, 0, 9, 9]);
    let shorter = TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_block([0]);
    let empty = TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_block([]);

    let longer = open_bytes(dir.path(), &v1(longer))?;
    let shorter = open_bytes(dir.path(), &v1(shorter))?;
    let empty = open_bytes(dir.path(), &v1(empty))?;

    assert_eq!(
        longer.issues(),
        [(0, EntryIssue::EncryptionBlockSize { size: 8 })]
    );
    assert!(!longer.entries()[0].is_encrypted(), "value 0 is plain");
    assert_eq!(
        shorter.issues(),
        [(0, EntryIssue::EncryptionBlockSize { size: 5 })]
    );
    assert!(shorter.entries()[0].is_encrypted(), "no value is encrypted");
    assert_eq!(
        empty.issues(),
        [(0, EntryIssue::EncryptionBlockSize { size: 4 })]
    );
    assert!(empty.entries()[0].is_encrypted());
    Ok(())
}

#[test]
fn a_v2_encryption_field_above_one_is_reported_and_read_as_encrypted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let entry = TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()).with_encryption_field(7);

    let archive = open_bytes(dir.path(), &v2(entry))?;

    assert_eq!(
        archive.issues(),
        [(0, EntryIssue::UnknownEncryption { value: 7 })]
    );
    assert!(archive.entries()[0].is_encrypted());
    Ok(())
}

#[test]
fn a_v2_entry_whose_data_leaves_the_file_is_reported_and_fails_only_when_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = ArchiveBuilder::v2()
        .entry(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()))
        .entry(TestEntry::stored("Data\\b.xml", b"<b/>".to_vec()))
        .build();
    let mut broken = bytes.clone();
    let directory = usize::try_from(u64::from_le_bytes(
        bytes[bytes.len() - V2_TRAILER_LEN + 16..][..8].try_into()?,
    ))?;
    put_u64(&mut broken, directory + 204 + 0x1A, 1 << 20)?;

    let archive = open_bytes(dir.path(), &broken)?;

    assert_eq!(archive.issues(), [(1, EntryIssue::DataOutsideFile)]);
    assert_eq!(archive.entries().len(), 2, "both Entries stay listed");
    assert_eq!(read_all(&archive, 0)?, b"<a/>", "the sound Entry reads");
    let entry = archive.entries().get(1).ok_or("Entry missing")?;
    assert!(
        matches!(archive.open_entry(entry), Err(ArchiveError::Corrupt { .. })),
        "the broken Entry fails with a typed error"
    );
    Ok(())
}

#[test]
fn a_v2_entry_whose_data_overlaps_the_directory_is_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut bytes = v2(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()));
    let directory = usize::try_from(u64::from_le_bytes(
        bytes[bytes.len() - V2_TRAILER_LEN + 16..][..8].try_into()?,
    ))?;
    put_u64(&mut bytes, directory + 0x0A, u64::MAX)?;

    let archive = open_bytes(dir.path(), &bytes)?;

    assert_eq!(archive.issues(), [(0, EntryIssue::DataOutsideFile)]);
    Ok(())
}

#[test]
fn a_v1_path_that_is_not_utf8_is_decoded_lossily_and_reported_with_its_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let raw = b"Data\\caf\xE9.txt".to_vec();
    let entry =
        TestEntry::stored("Data\\placeholder", b"<a/>".to_vec()).with_path_bytes(raw.clone());

    let archive = open_bytes(dir.path(), &v1(entry))?;

    let listed = archive.entries().first().ok_or("Entry missing")?;
    assert_eq!(listed.path(), "Data\\caf\u{FFFD}.txt");
    assert_eq!(
        archive.issues(),
        [(
            0,
            EntryIssue::PathNotUtf8 {
                raw: raw.clone().into()
            }
        )]
    );
    assert_eq!(archive.raw_path(listed), Some(raw.as_slice()));
    assert_eq!(read_all(&archive, 0)?, b"<a/>", "the Entry reads");
    Ok(())
}

#[test]
fn a_v2_path_that_is_not_utf8_is_decoded_lossily_and_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let raw = b"Data/\xFF\xFE.txt".to_vec();
    let entry = TestEntry::stored("x", b"<a/>".to_vec()).with_path_bytes(raw.clone());

    let archive = open_bytes(dir.path(), &v2(entry))?;

    let listed = archive.entries().first().ok_or("Entry missing")?;
    assert_eq!(listed.path(), "Data\\\u{FFFD}\u{FFFD}.txt");
    assert_eq!(archive.raw_path(listed), Some(raw.as_slice()));
    Ok(())
}

#[test]
fn entries_whose_lossy_paths_are_equal_keep_their_own_raw_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = ArchiveBuilder::new()
        .entry(TestEntry::stored("x", b"1".to_vec()).with_path_bytes(b"a\xE9".to_vec()))
        .entry(TestEntry::stored("Data\\ok.txt", b"2".to_vec()))
        .entry(TestEntry::stored("x", b"3".to_vec()).with_path_bytes(b"a\xE8".to_vec()))
        .build();

    let archive = open_bytes(dir.path(), &bytes)?;

    let entries = archive.entries();
    assert_eq!(entries[0].path(), entries[2].path());
    assert_eq!(archive.raw_path(&entries[0]), Some(b"a\xE9".as_slice()));
    assert_eq!(archive.raw_path(&entries[1]), None);
    assert_eq!(archive.raw_path(&entries[2]), Some(b"a\xE8".as_slice()));
    let positions: Vec<usize> = archive.issues().iter().map(|(index, _)| *index).collect();
    assert_eq!(positions, [0, 2]);
    Ok(())
}

#[test]
fn container_issues_follow_their_entries_past_skipped_folders() -> TestResult {
    let dir = tempfile::tempdir()?;
    let container = TestContainer::new()
        .entry(TestEntry::stored("room\\", Vec::new()))
        .entry(TestEntry::stored("a", b"1".to_vec()).with_path_bytes(b"room\\\xFF".to_vec()))
        .entry(TestEntry::stored("room\\ok.xml", b"2".to_vec()));
    let bytes = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\room.socpak", container.build()))
        .build();
    let archive = open_bytes(dir.path(), &bytes)?;
    let holder: &Entry = archive.entries().first().ok_or("Entry missing")?;

    let opened = archive.open_container(holder)?;

    assert_eq!(opened.entries().len(), 2, "the folder Entry is left out");
    assert_eq!(opened.issues().len(), 1);
    assert_eq!(
        opened.issues()[0].0,
        0,
        "the issue moved to the first Entry"
    );
    assert!(opened.raw_path(&opened.entries()[0]).is_some());
    assert!(opened.raw_path(&opened.entries()[1]).is_none());
    Ok(())
}

#[test]
fn a_zip64_locator_before_other_bytes_is_found_by_searching_backwards() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut bytes = v1(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()));
    let end = last_position(&bytes, &END_SIGNATURE)?;
    bytes.splice(end..end, [0xAB; 37]);

    let archive = open_bytes(dir.path(), &bytes)?;

    assert_eq!(archive.entries().len(), 1);
    assert_eq!(read_all(&archive, 0)?, b"<a/>");
    Ok(())
}

#[test]
fn a_locator_pointing_at_no_zip64_end_record_is_ignored() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut bytes = v1(TestEntry::stored("Data\\a.xml", b"<a/>".to_vec()));
    let end = last_position(&bytes, &END_SIGNATURE)?;
    bytes.splice(end..end, [0xAB; 37]);
    let zip64_end = last_position(&bytes, &ZIP64_END_SIGNATURE)?;
    bytes[zip64_end] = 0;

    let result = open_bytes(dir.path(), &bytes);

    assert!(
        matches!(result, Err(ref error) if error.to_string().contains("ZIP64")),
        "the markers of the end record have no ZIP64 locator to follow: {result:?}"
    );
    Ok(())
}
