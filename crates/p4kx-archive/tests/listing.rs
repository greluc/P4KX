//! Listing the Entries of an Archive through the public API.

use std::error::Error;

use p4kx_archive::{Archive, Compression};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn lists_every_entry_with_path_sizes_method_and_encryption() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\Config\\defaultProfile.xml",
            b"<Profile/>".to_vec(),
        ))
        .entry(TestEntry::stored("Data\\Scripts\\empty.lua", Vec::new()).flagged_encrypted())
        .write_to(&path)?;

    let archive = Archive::open(&path)?;

    let listed: Vec<_> = archive
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.path(),
                entry.compressed_size(),
                entry.uncompressed_size(),
                entry.compression(),
                entry.is_encrypted(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        vec![
            (
                "Data\\Libs\\Config\\defaultProfile.xml",
                10,
                10,
                Compression::Store,
                false
            ),
            ("Data\\Scripts\\empty.lua", 0, 0, Compression::Store, true),
        ],
        "every Entry is listed in central-directory order"
    );
    Ok(())
}

#[test]
fn lists_more_entries_than_a_16_bit_count_can_hold() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let count = 70_000;
    (0..count)
        .fold(ArchiveBuilder::new(), |builder, index| {
            builder.entry(TestEntry::stored(
                format!("Data\\Many\\{index:05}.txt"),
                Vec::new(),
            ))
        })
        .write_to(&path)?;

    let archive = Archive::open(&path)?;

    assert_eq!(
        archive.entries().len(),
        count,
        "all Entries beyond 65535 are listed"
    );
    assert_eq!(
        archive.entries().last().map(p4kx_archive::Entry::path),
        Some("Data\\Many\\69999.txt"),
        "the last Entry is read from the ZIP64 central directory"
    );
    Ok(())
}
