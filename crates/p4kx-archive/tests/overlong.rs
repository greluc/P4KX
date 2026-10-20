//! Entry data longer than its uncompressed size, cut or rejected.

use std::error::Error;
use std::io::{self, Read};

use p4kx_archive::{Archive, DataError};
use p4kx_testkit::{ArchiveBuilder, TestEntry, test_key};

type TestResult = Result<(), Box<dyn Error>>;

fn sample() -> Vec<u8> {
    (0..100_u8).collect()
}

fn read(entry: TestEntry, reject: bool) -> Result<io::Result<Vec<u8>>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new().entry(entry).write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(test_key()))?;
    let entry = archive.entries().first().ok_or("Entry missing")?;
    let mut reader = archive.open_entry(entry)?;
    if reject {
        reader = reader.reject_overlong();
    }
    let mut data = Vec::new();
    Ok(reader.read_to_end(&mut data).map(|_| data))
}

fn overlong() -> TestEntry {
    TestEntry::zstd("Data\\a.ini", sample()).with_uncompressed_size(60)
}

#[test]
fn overlong_data_is_cut_by_default() -> TestResult {
    let data = read(overlong(), false)??;
    assert_eq!(data.len(), 60, "cut to the uncompressed size");
    Ok(())
}

#[test]
fn overlong_data_is_an_error_when_rejected() -> TestResult {
    let error = read(overlong(), true)?.err().ok_or("reading succeeded")?;
    let data_error = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<DataError>())
        .ok_or("a DataError")?;
    assert!(
        matches!(data_error, DataError::TooLong { expected: 60 }),
        "too long: {data_error}"
    );
    Ok(())
}

#[test]
fn data_of_exactly_the_size_and_aes_padding_are_accepted_when_rejecting() -> TestResult {
    let exact = read(TestEntry::zstd("Data\\a.ini", sample()), true)??;
    assert_eq!(exact, sample(), "exact data");
    let padded = read(
        TestEntry::stored("Data\\a.ini", b"key=".to_vec()).encrypted(),
        true,
    )??;
    assert_eq!(padded, b"key=", "zero padding below one AES block");
    Ok(())
}
