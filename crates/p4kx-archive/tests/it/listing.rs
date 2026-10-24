// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Listing the Entries of an Archive through the public API.

use std::error::Error;

use p4kx_archive::{Archive, Compression, Entry};
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
        archive.entries().last().map(Entry::path),
        Some("Data\\Many\\69999.txt"),
        "the last Entry is read from the ZIP64 central directory"
    );
    Ok(())
}

#[test]
fn every_entry_carries_the_crc32c_of_its_uncompressed_data_in_both_layouts() -> TestResult {
    let dir = tempfile::tempdir()?;
    for (name, builder) in [("v1", ArchiveBuilder::new()), ("v2", ArchiveBuilder::v2())] {
        let path = dir.path().join(format!("{name}.p4k"));
        builder
            .entry(TestEntry::stored("Data\\check.txt", b"123456789".to_vec()))
            .entry(TestEntry::zstd("Data\\secret.txt", b"123456789".to_vec()).encrypted())
            .entry(TestEntry::stored("Data\\empty.txt", Vec::new()))
            .write_to(&path)?;

        let archive = Archive::open(&path)?;

        let crcs: Vec<_> = archive.entries().iter().map(Entry::crc32c).collect();
        assert_eq!(
            crcs,
            vec![Some(0xE306_9283), Some(0xE306_9283), Some(0)],
            "{name}: the CRC-32C of the data after decoding"
        );
    }
    Ok(())
}
