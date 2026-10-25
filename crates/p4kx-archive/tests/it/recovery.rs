// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Rebuilding the directory of damaged Archives, only when asked for.

use std::error::Error;
use std::io::Read;
use std::path::Path;

use p4kx_archive::{Archive, ArchiveError, DataError, Entry, EntryIssue, OpenOptions};
use p4kx_testkit::{ArchiveBuilder, TestEntry, test_key};

type TestResult = Result<(), Box<dyn Error>>;
type Variants = Vec<(&'static str, Vec<u8>)>;

const CENTRAL: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];
const END: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
const ZIP64_END: [u8; 4] = [0x50, 0x4B, 0x06, 0x06];
const PAYLOADS: [(&str, &[u8]); 3] = [
    ("Data\\a.xml", b"<a>first payload</a>"),
    ("Data\\b.xml", b"<b>second payload, a little longer</b>"),
    ("Data\\c.xml", b"<c>third payload</c>"),
];

fn archive_bytes() -> Vec<u8> {
    let mut builder = ArchiveBuilder::new();
    for (path, data) in PAYLOADS {
        builder = builder.entry(TestEntry::stored(path, data).with_sha256());
    }
    builder
        .entry(
            TestEntry::stored("Data\\secret.bin", b"secret payload, over one AES block")
                .encrypted(),
        )
        .build()
}

fn find(bytes: &[u8], pattern: [u8; 4], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(4)
        .position(|window| window == pattern)
        .map(|at| at + from)
}

fn rfind(bytes: &[u8], pattern: [u8; 4]) -> Option<usize> {
    bytes.windows(4).rposition(|window| window == pattern)
}

fn write(dir: &Path, bytes: &[u8]) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    std::fs::write(&path, bytes)?;
    Ok(path)
}

fn recovering(path: &Path) -> Result<Archive, ArchiveError> {
    Archive::open_with(path, OpenOptions::new().recover(true).key(Some(test_key())))
}

fn assert_recovered(archive: &Archive, expected: usize) {
    assert_eq!(archive.entries().len(), expected, "recovered Entries");
    assert!(
        archive
            .issues()
            .first()
            .is_some_and(|(index, issue)| *index == 0 && *issue == EntryIssue::Recovered),
        "the first issue says the Archive was recovered: {:?}",
        archive.issues()
    );
}

fn assert_all_verified(archive: &Archive) -> TestResult {
    for entry in archive.entries() {
        let mut data = Vec::new();
        archive
            .open_entry_verified(entry)?
            .read_to_end(&mut data)
            .map_err(|error| format!("{}: {error}", entry.path()))?;
        if let Some((_, expected)) = PAYLOADS.iter().find(|(path, _)| *path == entry.path()) {
            assert_eq!(&data, expected, "{}", entry.path());
        }
    }
    Ok(())
}

fn damaged_variants() -> Result<Variants, Box<dyn Error>> {
    let bytes = archive_bytes();
    let directory = find(&bytes, CENTRAL, 0).ok_or("no directory")?;
    let end = rfind(&bytes, END).ok_or("no end record")?;
    let zip64_end = rfind(&bytes, ZIP64_END).ok_or("no ZIP64 end record")?;
    let mut zeroed = bytes.clone();
    zeroed
        .get_mut(end..end + 22)
        .ok_or("end record out of range")?
        .fill(0);
    let mut no_zip64 = bytes.clone();
    no_zip64
        .get_mut(zip64_end..end)
        .ok_or("ZIP64 records out of range")?
        .fill(0);
    let mut cut_end = bytes.clone();
    cut_end.truncate(end + 9);
    let mut cut_directory = bytes.clone();
    cut_directory.truncate(directory + 120);
    Ok(vec![
        ("zeroed end record", zeroed),
        ("zeroed ZIP64 records", no_zip64),
        ("truncated end record", cut_end),
        (
            "no end records",
            bytes.get(..zip64_end).ok_or("range")?.to_vec(),
        ),
        ("truncated directory", cut_directory),
    ])
}

#[test]
fn damaged_archives_do_not_open_by_default() -> TestResult {
    for (name, bytes) in damaged_variants()? {
        let dir = tempfile::tempdir()?;
        let path = write(dir.path(), &bytes)?;

        let result = Archive::open(&path);

        assert!(result.is_err(), "{name} must not open without recovery");
    }
    Ok(())
}

#[test]
fn archives_without_valid_end_records_are_rebuilt_on_request() -> TestResult {
    for (name, bytes) in damaged_variants()? {
        if name == "truncated directory" {
            continue;
        }
        let dir = tempfile::tempdir()?;
        let path = write(dir.path(), &bytes)?;

        let archive = recovering(&path).map_err(|error| format!("{name}: {error}"))?;

        assert_recovered(&archive, 4);
        assert_all_verified(&archive)?;
        assert!(
            archive
                .entries()
                .iter()
                .all(|entry| entry.crc32c().is_some())
        );
    }
    Ok(())
}

#[test]
fn a_truncated_directory_keeps_the_records_before_the_cut() -> TestResult {
    let bytes = archive_bytes();
    let directory = find(&bytes, CENTRAL, 0).ok_or("no directory")?;
    let second = find(&bytes, CENTRAL, directory + 4).ok_or("no second header")?;
    let third = find(&bytes, CENTRAL, second + 4).ok_or("no third header")?;
    let dir = tempfile::tempdir()?;
    let path = write(dir.path(), bytes.get(..third + 30).ok_or("range")?)?;

    let archive = recovering(&path)?;

    assert_recovered(&archive, 2);
    assert_all_verified(&archive)?;
    Ok(())
}

#[test]
fn a_damaged_central_directory_header_costs_only_that_record() -> TestResult {
    let mut bytes = archive_bytes();
    let directory = find(&bytes, CENTRAL, 0).ok_or("no directory")?;
    let second = find(&bytes, CENTRAL, directory + 4).ok_or("no second header")?;
    bytes.get_mut(second..second + 4).ok_or("range")?.fill(0xEE);
    let dir = tempfile::tempdir()?;
    let path = write(dir.path(), &bytes)?;
    assert!(Archive::open(&path).is_err(), "the damage is seen by open");

    let archive = recovering(&path)?;

    assert_recovered(&archive, 3);
    let paths: Vec<_> = archive.entries().iter().map(Entry::path).collect();
    assert_eq!(paths, ["Data\\a.xml", "Data\\c.xml", "Data\\secret.bin"]);
    assert_all_verified(&archive)?;
    Ok(())
}

#[test]
fn an_intact_archive_opens_as_usual_with_recovery_on() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = write(dir.path(), &archive_bytes())?;

    let archive = recovering(&path)?;

    assert_eq!(archive.entries().len(), 4);
    assert_eq!(archive.issue_count(), 0, "{:?}", archive.issues());
    Ok(())
}

#[test]
fn a_record_whose_local_header_has_another_name_is_dropped() -> TestResult {
    let mut bytes = archive_bytes();
    let local = find(&bytes, [0x50, 0x4B, 0x03, 0x04], 0)
        .or_else(|| find(&bytes, [0x50, 0x4B, 0x03, 0x14], 0))
        .ok_or("no local header")?;
    let name_at = local + 30;
    *bytes.get_mut(name_at + 5).ok_or("range")? = b'Z';
    let end = rfind(&bytes, END).ok_or("no end record")?;
    bytes.get_mut(end..end + 4).ok_or("range")?.fill(0);
    let dir = tempfile::tempdir()?;
    let path = write(dir.path(), &bytes)?;

    let archive = recovering(&path)?;

    assert_recovered(&archive, 3);
    assert!(
        archive
            .entries()
            .iter()
            .all(|entry| entry.path() != "Data\\a.xml")
    );
    Ok(())
}

#[test]
fn recovered_entries_are_verified_like_any_other() -> TestResult {
    let mut bytes = archive_bytes();
    let payload = bytes
        .windows(10)
        .position(|window| window == b"second pay")
        .ok_or("no payload")?;
    *bytes.get_mut(payload).ok_or("range")? ^= 0xFF;
    let end = rfind(&bytes, END).ok_or("no end record")?;
    bytes.get_mut(end..end + 4).ok_or("range")?.fill(0);
    let dir = tempfile::tempdir()?;
    let path = write(dir.path(), &bytes)?;
    let archive = recovering(&path)?;
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path() == "Data\\b.xml")
        .ok_or("no b.xml")?;

    let mut data = Vec::new();
    let error = archive
        .open_entry_verified(entry)?
        .read_to_end(&mut data)
        .err()
        .ok_or("the damaged payload read cleanly")?;

    let inner = error.into_inner().ok_or("no inner error")?;
    assert!(matches!(
        inner.downcast_ref::<DataError>(),
        Some(DataError::ChecksumMismatch { .. })
    ));
    Ok(())
}

#[test]
fn files_without_records_keep_their_error() -> TestResult {
    for bytes in [
        Vec::new(),
        b"plain text, no records at all".to_vec(),
        CENTRAL.repeat(300_000),
        vec![0; 1 << 20],
    ] {
        let dir = tempfile::tempdir()?;
        let path = write(dir.path(), &bytes)?;

        let result = recovering(&path);

        assert!(
            matches!(result, Err(ArchiveError::NotAnArchive { .. })),
            "{result:?}"
        );
    }
    Ok(())
}
