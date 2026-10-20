//! Checking the SHA-256 of the stored bytes of an Entry while it is read.

use std::error::Error;
use std::io::{self, Read};

use p4kx_archive::{Archive, Checksum, DataError};
use p4kx_testkit::{ArchiveBuilder, TestEntry, test_key};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const DATA: &[u8] = b"<sha-256 payload, long enough to span several AES blocks, and then some/>";

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn builders() -> [ArchiveBuilder; 2] {
    [ArchiveBuilder::new(), ArchiveBuilder::v2()]
}

fn open(builder: &ArchiveBuilder) -> Result<(tempfile::TempDir, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(test_key()))?;
    Ok((dir, archive))
}

fn read(archive: &Archive, verified: bool) -> Result<io::Result<Vec<u8>>, Box<dyn Error>> {
    let entry = archive.entries().first().ok_or("no Entry")?;
    let mut data = Vec::new();
    let mut reader = if verified {
        archive.open_entry_verified(entry)?
    } else {
        archive.open_entry(entry)?
    };
    Ok(reader.read_to_end(&mut data).map(|_| data))
}

fn mismatch(result: io::Result<Vec<u8>>) -> Result<(Checksum, Checksum), Box<dyn Error>> {
    let error = result.err().ok_or("reading succeeded")?;
    let inner = error.into_inner().ok_or("no inner error")?;
    match *inner
        .downcast::<DataError>()
        .map_err(|other| format!("{other}"))?
    {
        DataError::ChecksumMismatch {
            expected, actual, ..
        } => Ok((expected, actual)),
        other => Err(format!("another error: {other}").into()),
    }
}

fn kinds() -> Vec<(TestEntry, &'static [u8])> {
    let name = r"Data\a.bin";
    vec![
        (TestEntry::stored(name, DATA), DATA),
        (TestEntry::deflated(name, DATA), DATA),
        (TestEntry::zstd(name, DATA), DATA),
        (TestEntry::zstd_without_magic(name, DATA), DATA),
        (TestEntry::stored(name, DATA).encrypted(), DATA),
        (TestEntry::deflated(name, DATA).encrypted(), DATA),
        (TestEntry::zstd(name, DATA).encrypted(), DATA),
        (TestEntry::stored(name, Vec::new()), b""),
        (TestEntry::zstd(name, Vec::new()).encrypted(), b""),
    ]
}

fn sibling() -> TestEntry {
    TestEntry::stored("Data\\sibling.bin", DATA).encrypted()
}

#[test]
fn an_entry_states_the_sha_256_of_its_stored_bytes() -> TestResult {
    for builder in builders() {
        let entry = TestEntry::zstd("Data\\a.bin", DATA).encrypted();
        let digest = sha(entry.payload());
        let (_dir, archive) = open(&builder.entry(entry.with_sha256()))?;

        let stated = archive.entries().first().ok_or("no Entry")?.sha256();

        assert_eq!(stated, Some(digest), "the digest of the stored bytes");
    }
    Ok(())
}

#[test]
fn an_entry_without_a_digest_states_none_and_is_not_checked() -> TestResult {
    for builder in builders() {
        let (_dir, archive) = open(&builder.entry(TestEntry::stored("Data\\a.bin", DATA)))?;

        assert_eq!(
            archive.entries().first().ok_or("no Entry")?.sha256(),
            None,
            "an all-zero digest states none"
        );
        assert_eq!(read(&archive, true)??, DATA, "reads and verifies");
    }
    Ok(())
}

#[test]
fn a_correct_digest_passes_for_every_kind_in_both_layouts() -> TestResult {
    for builder in builders() {
        for (entry, expected) in kinds() {
            let (_dir, archive) =
                open(&builder.clone().entry(entry.with_sha256()).entry(sibling()))?;

            assert_eq!(read(&archive, true)??, expected, "the data reads back");
        }
    }
    Ok(())
}

#[test]
fn a_wrong_digest_is_reported_for_every_kind_in_both_layouts() -> TestResult {
    let wrong = [0x5A; 32];
    for builder in builders() {
        for (entry, _) in kinds() {
            let digest = sha(entry.payload());
            let (_dir, archive) = open(
                &builder
                    .clone()
                    .entry(entry.with_sha256_value(wrong))
                    .entry(sibling()),
            )?;

            let (expected, actual) = mismatch(read(&archive, true)?)?;

            assert_eq!(expected, Checksum::Sha256(wrong), "the stated digest");
            assert_eq!(actual, Checksum::Sha256(digest), "the computed digest");
        }
    }
    Ok(())
}

#[test]
fn a_digest_of_only_the_data_fails_when_stored_bytes_follow_it() -> TestResult {
    for builder in builders() {
        let entry = TestEntry::stored("Data\\a.bin", DATA);
        let digest = sha(entry.payload());
        let mut payload = entry.payload().to_vec();
        payload.extend_from_slice(&[0xAA; 100]);
        let tampered = entry.with_payload(payload).with_sha256_value(digest);
        let (_dir, archive) = open(&builder.entry(tampered))?;

        let (expected, actual) = mismatch(read(&archive, true)?)?;

        assert_eq!(expected, Checksum::Sha256(digest), "the stated digest");
        assert_ne!(actual, expected, "the bytes after the data are hashed too");
    }
    Ok(())
}

#[test]
fn stored_bytes_beyond_the_data_are_covered_by_the_digest() -> TestResult {
    for builder in builders() {
        let entry = TestEntry::stored("Data\\a.bin", DATA);
        let mut payload = entry.payload().to_vec();
        payload.resize(payload.len() + 20_000, 0xAA);
        let (_dir, archive) = open(&builder.entry(entry.with_payload(payload).with_sha256()))?;

        assert_eq!(read(&archive, true)??, DATA, "the longer stored bytes pass");
    }
    Ok(())
}

#[test]
fn the_crc_is_reported_first_when_both_checks_fail() -> TestResult {
    let entry = TestEntry::stored("Data\\a.bin", DATA)
        .with_crc32c(0x1234_5678)
        .with_sha256_value([0x5A; 32]);
    let (_dir, archive) = open(&ArchiveBuilder::new().entry(entry))?;

    let (expected, _) = mismatch(read(&archive, true)?)?;

    assert_eq!(expected, Checksum::Crc32c(0x1234_5678), "the CRC-32C first");
    Ok(())
}

#[test]
fn reading_without_verification_ignores_the_digest() -> TestResult {
    for builder in builders() {
        let entry = TestEntry::deflated("Data\\a.bin", DATA).with_sha256_value([0x5A; 32]);
        let (_dir, archive) = open(&builder.entry(entry))?;

        assert_eq!(read(&archive, false)??, DATA, "open_entry does not check");
    }
    Ok(())
}
