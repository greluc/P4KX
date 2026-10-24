// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading compressed and encrypted Entries through the public API.

use std::error::Error;
use std::io::{self, Read};

use p4kx_archive::{Archive, ArchiveError, Compression, DataError};
use p4kx_testkit::{ArchiveBuilder, TestEntry, test_key};

type TestResult = Result<(), Box<dyn Error>>;

fn sample(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| u8::try_from((index * 7 + index / 4096) % 251).unwrap_or(0))
        .collect()
}

fn open(builder: &ArchiveBuilder) -> Result<(tempfile::TempDir, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(test_key()))?;
    Ok((dir, archive))
}

fn read_entry(archive: &Archive, index: usize) -> Result<io::Result<Vec<u8>>, ArchiveError> {
    let Some(entry) = archive.entries().get(index) else {
        return Ok(Err(io::Error::other("Entry missing")));
    };
    let mut reader = archive.open_entry(entry)?;
    let mut data = Vec::new();
    Ok(reader.read_to_end(&mut data).map(|_| data))
}

fn data_error(result: io::Result<Vec<u8>>) -> Result<DataError, Box<dyn Error>> {
    let error = result.err().ok_or("reading succeeded")?;
    assert_eq!(
        error.kind(),
        io::ErrorKind::InvalidData,
        "error kind of {error}"
    );
    let inner = error.into_inner().ok_or("no inner error")?;
    Ok(*inner
        .downcast::<DataError>()
        .map_err(|other| format!("{other}"))?)
}

#[test]
fn reads_compressed_and_encrypted_entries() -> TestResult {
    let big = sample(3 << 20);
    let cases = [
        TestEntry::deflated("Data\\a.xml", b"<deflate/>".to_vec()),
        TestEntry::zstd("Data\\b.xml", b"<zstd/>".to_vec()),
        TestEntry::zstd_without_magic("Data\\c.xml", b"<plain/>".to_vec()),
        TestEntry::stored("Data\\d.xml", b"<stored secret/>".to_vec()).encrypted(),
        TestEntry::deflated("Data\\e.xml", b"<deflate secret/>".to_vec()).encrypted(),
        TestEntry::zstd("Data\\f.xml", b"<zstd secret/>".to_vec()).encrypted(),
        TestEntry::zstd_without_magic("Data\\g.xml", b"<plain secret/>".to_vec()).encrypted(),
        TestEntry::deflated("Data\\big.deflate", big.clone()),
        TestEntry::zstd("Data\\big.zstd", big.clone()).encrypted(),
        TestEntry::zstd("Data\\empty.zstd", Vec::new()).encrypted(),
    ];
    let expected: [&[u8]; 10] = [
        b"<deflate/>",
        b"<zstd/>",
        b"<plain/>",
        b"<stored secret/>",
        b"<deflate secret/>",
        b"<zstd secret/>",
        b"<plain secret/>",
        &big,
        &big,
        b"",
    ];
    let builder = cases
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry);
    let (_dir, archive) = open(&builder)?;

    for (index, expected) in expected.iter().enumerate() {
        let data = read_entry(&archive, index)??;
        assert!(
            data == *expected,
            "Entry {index} reads back its original {} bytes, got {}",
            expected.len(),
            data.len()
        );
    }
    Ok(())
}

#[test]
fn entries_report_their_compression_and_encryption() -> TestResult {
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::deflated("a", b"a".to_vec()))
        .entry(TestEntry::zstd("b", b"b".to_vec()).encrypted());
    let (_dir, archive) = open(&builder)?;

    let summary: Vec<_> = archive
        .entries()
        .iter()
        .map(|entry| (entry.compression(), entry.is_encrypted()))
        .collect();

    assert_eq!(
        summary,
        vec![(Compression::Deflate, false), (Compression::Zstd, true)],
        "compression and encryption of each Entry"
    );
    Ok(())
}

#[test]
fn corrupt_deflate_data_fails_with_a_deflate_error() -> TestResult {
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::deflated("Data\\a.xml", sample(1000)).with_payload(vec![0xFF; 64]));
    let (_dir, archive) = open(&builder)?;

    let error = data_error(read_entry(&archive, 0)?)?;

    assert!(
        matches!(
            error,
            DataError::Corrupt {
                compression: Compression::Deflate,
                ..
            }
        ),
        "corrupt deflate data: {error}"
    );
    Ok(())
}

#[test]
fn corrupt_zstd_data_fails_with_a_zstd_error() -> TestResult {
    let entry = TestEntry::zstd("Data\\a.xml", sample(100_000));
    let mut payload = entry.payload().to_vec();
    payload.truncate(12);
    payload.extend_from_slice(&[0xFF; 64]);
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\anchor.bin", b"anchor".to_vec()).encrypted())
        .entry(entry.with_payload(payload).encrypted());
    let (_dir, archive) = open(&builder)?;

    let error = data_error(read_entry(&archive, 1)?)?;

    assert!(
        matches!(
            error,
            DataError::Corrupt {
                compression: Compression::Zstd,
                ..
            }
        ),
        "corrupt zstd data: {error}"
    );
    Ok(())
}

#[test]
fn data_shorter_than_the_uncompressed_size_fails() -> TestResult {
    let entry = TestEntry::zstd_without_magic("Data\\a.xml", sample(100));
    let payload = entry
        .payload()
        .get(..40)
        .ok_or("payload too short")?
        .to_vec();
    let builder = ArchiveBuilder::new().entry(entry.with_payload(payload));
    let (_dir, archive) = open(&builder)?;

    let error = data_error(read_entry(&archive, 0)?)?;

    assert!(
        matches!(
            error,
            DataError::TooShort {
                actual: 40,
                expected: 100
            }
        ),
        "short data: {error}"
    );
    Ok(())
}

#[test]
fn encrypted_data_of_partial_blocks_is_reported_as_corrupt() -> TestResult {
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\secret.xml", b"<s/>".to_vec()).flagged_encrypted());
    let (_dir, archive) = open(&builder)?;
    let entry = archive.entries().first().ok_or("Entry missing")?;

    let result = archive.open_entry(entry);

    assert!(
        matches!(result, Err(ArchiveError::Corrupt { ref reason, .. }) if reason.contains("AES")),
        "encrypted data must be whole AES blocks: {result:?}"
    );
    Ok(())
}
