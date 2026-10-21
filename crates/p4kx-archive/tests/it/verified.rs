//! Checking the checksum of an Entry while it is read.

use std::error::Error;
use std::io::{self, Read};

use p4kx_archive::{Archive, Checksum, DataError};
use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry, test_key};

type TestResult = Result<(), Box<dyn Error>>;

const DATA: &[u8] = b"<verified payload, long enough to span several AES blocks/>";

fn open(builder: &ArchiveBuilder) -> Result<(tempfile::TempDir, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(test_key()))?;
    Ok((dir, archive))
}

fn with_checked_sibling(entry: TestEntry) -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(entry)
        .entry(TestEntry::stored("Data\\sibling.bin", DATA).encrypted())
}

fn read_verified(archive: &Archive) -> Result<io::Result<Vec<u8>>, Box<dyn Error>> {
    let entry = archive.entries().first().ok_or("no Entry")?;
    let mut data = Vec::new();
    Ok(archive
        .open_entry_verified(entry)?
        .read_to_end(&mut data)
        .map(|_| data))
}

fn mismatch(result: io::Result<Vec<u8>>) -> Result<DataError, Box<dyn Error>> {
    let error = result.err().ok_or("reading succeeded")?;
    assert_eq!(error.kind(), io::ErrorKind::InvalidData, "kind of {error}");
    let inner = error.into_inner().ok_or("no inner error")?;
    Ok(*inner
        .downcast::<DataError>()
        .map_err(|other| format!("{other}"))?)
}

fn flipped(entry: &TestEntry) -> TestEntry {
    let mut payload = entry.payload().to_vec();
    if let Some(byte) = payload.first_mut() {
        *byte ^= 0x01;
    }
    entry.clone().with_payload(payload)
}

#[test]
fn a_correct_entry_of_every_kind_passes() -> TestResult {
    let kinds = [
        (TestEntry::stored("Data\\a.bin", DATA), DATA),
        (TestEntry::deflated("Data\\a.bin", DATA), DATA),
        (TestEntry::zstd("Data\\a.bin", DATA), DATA),
        (TestEntry::stored("Data\\a.bin", DATA).encrypted(), DATA),
        (TestEntry::deflated("Data\\a.bin", DATA).encrypted(), DATA),
        (TestEntry::zstd("Data\\a.bin", DATA).encrypted(), DATA),
        (TestEntry::stored("Data\\a.bin", Vec::new()), b"".as_slice()),
    ];
    for (entry, expected) in kinds {
        let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry))?;
        let data = read_verified(&archive)??;
        assert_eq!(data, expected, "the data reads back unchanged");
    }
    Ok(())
}

#[test]
fn a_flipped_byte_in_a_stored_entry_is_reported() -> TestResult {
    let (_dir, archive) =
        open(&ArchiveBuilder::new().entry(flipped(&TestEntry::stored("Data\\a.bin", DATA))))?;

    let DataError::ChecksumMismatch {
        path,
        expected,
        actual,
    } = mismatch(read_verified(&archive)?)?
    else {
        return Err("another error".into());
    };

    assert_eq!(path, "Data\\a.bin", "the error names the Entry");
    assert_eq!(
        expected,
        Checksum::Crc32c(crc32c::crc32c(DATA)),
        "the stored value"
    );
    assert_ne!(actual, expected, "the computed value differs");
    Ok(())
}

#[test]
fn a_flipped_byte_in_an_encrypted_entry_is_reported() -> TestResult {
    let entry = TestEntry::stored("Data\\a.bin", DATA).encrypted();
    let (_dir, archive) = open(&with_checked_sibling(flipped(&entry)))?;

    let error = mismatch(read_verified(&archive)?)?;

    assert!(
        matches!(error, DataError::ChecksumMismatch { .. }),
        "got {error}"
    );
    Ok(())
}

#[test]
fn a_wrong_stored_checksum_is_reported_for_deflate_and_zstd() -> TestResult {
    for entry in [
        TestEntry::deflated("Data\\a.bin", DATA),
        TestEntry::zstd("Data\\a.bin", DATA),
        TestEntry::zstd("Data\\a.bin", DATA).encrypted(),
    ] {
        let (_dir, archive) = open(&with_checked_sibling(entry.with_crc32c(0x1234_5678)))?;

        let error = mismatch(read_verified(&archive)?)?;

        let DataError::ChecksumMismatch { expected, .. } = error else {
            return Err("another error".into());
        };
        assert_eq!(expected, Checksum::Crc32c(0x1234_5678), "the stored value");
    }
    Ok(())
}

#[test]
fn an_empty_entry_is_checked_too() -> TestResult {
    let entry = TestEntry::stored("Data\\empty.bin", Vec::new()).with_crc32c(7);
    let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry))?;

    let error = mismatch(read_verified(&archive)?)?;

    assert!(
        matches!(error, DataError::ChecksumMismatch { .. }),
        "got {error}"
    );
    Ok(())
}

#[test]
fn a_reader_that_stops_early_checks_nothing() -> TestResult {
    let entry = TestEntry::stored("Data\\a.bin", DATA).with_crc32c(1);
    let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry))?;
    let first = archive.entries().first().ok_or("no Entry")?;
    let mut head = [0; 4];

    archive.open_entry_verified(first)?.read_exact(&mut head)?;

    assert_eq!(&head, b"<ver", "the first bytes arrive");
    Ok(())
}

#[test]
fn reading_exactly_the_size_still_checks_at_the_last_byte() -> TestResult {
    let entry = TestEntry::stored("Data\\a.bin", DATA).with_crc32c(1);
    let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry))?;
    let first = archive.entries().first().ok_or("no Entry")?;
    let mut all = vec![0; DATA.len()];

    let error = archive
        .open_entry_verified(first)?
        .read_exact(&mut all)
        .err()
        .ok_or("read_exact succeeded")?;

    assert_eq!(error.kind(), io::ErrorKind::InvalidData, "kind of {error}");
    Ok(())
}

#[test]
fn the_plain_read_does_not_check() -> TestResult {
    let entry = TestEntry::stored("Data\\a.bin", DATA).with_crc32c(1);
    let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry))?;
    let first = archive.entries().first().ok_or("no Entry")?;
    let mut data = Vec::new();

    archive.open_entry(first)?.read_to_end(&mut data)?;

    assert_eq!(data, DATA, "open_entry stays unchecked");
    Ok(())
}

#[test]
fn a_container_entry_is_checked_against_its_plain_crc32() -> TestResult {
    let good = TestEntry::stored("room\\good.xml", "<Good/>");
    let bad = TestEntry::stored("room\\bad.xml", "<Bad/>").with_crc32(0xDEAD_BEEF);
    let container = TestContainer::new().entry(good).entry(bad);
    let (_dir, archive) = open(
        &ArchiveBuilder::new().entry(TestEntry::stored("Data\\room.socpak", container.build())),
    )?;
    let socpak = archive.open_container(archive.entries().first().ok_or("no Entry")?)?;

    let mut results = Vec::new();
    for entry in socpak.entries() {
        let mut data = Vec::new();
        results.push(
            socpak
                .open_entry_verified(entry)?
                .read_to_end(&mut data)
                .map(|_| data),
        );
    }
    let [good, bad] = <[_; 2]>::try_from(results).map_err(|_| "two Entries")?;

    assert_eq!(good?, b"<Good/>", "a correct container Entry passes");
    let DataError::ChecksumMismatch { expected, .. } = mismatch(bad)? else {
        return Err("another error".into());
    };
    assert_eq!(
        expected,
        Checksum::Crc32(0xDEAD_BEEF),
        "a container Entry carries a plain CRC-32"
    );
    Ok(())
}

fn stored_deflate_block(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let len = u16::try_from(data.len())?;
    let mut block = vec![0x01];
    block.extend_from_slice(&len.to_le_bytes());
    block.extend_from_slice(&(!len).to_le_bytes());
    block.extend_from_slice(data);
    Ok(block)
}

fn raw_zstd_frame(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let size = u8::try_from(data.len())?;
    let header = (u32::from(size) << 3) | 1;
    let mut frame = vec![0x28, 0xB5, 0x2F, 0xFD, 0x20, size];
    frame.extend_from_slice(header.to_le_bytes().get(..3).ok_or("header")?);
    frame.extend_from_slice(data);
    Ok(frame)
}

#[test]
fn a_flipped_data_byte_that_still_decodes_is_reported_for_deflate_and_zstd() -> TestResult {
    for (entry, framed) in [
        (
            TestEntry::deflated("Data\\a.bin", DATA),
            stored_deflate_block(DATA)?,
        ),
        (TestEntry::zstd("Data\\a.bin", DATA), raw_zstd_frame(DATA)?),
    ] {
        let (_dir, intact) =
            open(&ArchiveBuilder::new().entry(entry.clone().with_payload(framed.clone())))?;
        assert_eq!(read_verified(&intact)??, DATA, "the framed data decodes");
        let mut broken = framed;
        *broken.last_mut().ok_or("empty")? ^= 0x01;
        let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry.with_payload(broken)))?;

        let error = mismatch(read_verified(&archive)?)?;

        assert!(
            matches!(error, DataError::ChecksumMismatch { .. }),
            "got {error}"
        );
    }
    Ok(())
}
