// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The CIG hash block of a v1 directory record: a block of another length is reported, and the
//! last of several decides.

use std::error::Error;

use p4kx_archive::{Archive, EntryIssue};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn open(entry: TestEntry) -> Result<(tempfile::TempDir, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new().entry(entry).write_to(&path)?;
    let archive = Archive::open(&path)?;
    Ok((dir, archive))
}

fn entry() -> TestEntry {
    TestEntry::stored("Data\\a.bin", b"<a/>".to_vec())
}

fn stated(archive: &Archive) -> Result<Option<[u8; 32]>, Box<dyn Error>> {
    Ok(archive.entries().first().ok_or("no Entry")?.sha256())
}

#[test]
fn a_hash_block_of_another_length_is_reported_and_states_no_digest() -> TestResult {
    for (data, size) in [(vec![7_u8; 31], 35), (vec![7_u8; 33], 37), (Vec::new(), 4)] {
        let (_dir, archive) = open(entry().with_hash_block(data))?;

        assert_eq!(archive.issues(), [(0, EntryIssue::HashBlockSize { size })]);
        assert_eq!(stated(&archive)?, None, "no digest");
    }
    Ok(())
}

#[test]
fn a_hash_block_of_32_bytes_is_no_issue_even_when_all_zero() -> TestResult {
    for data in [vec![7_u8; 32], vec![0_u8; 32]] {
        let (_dir, archive) = open(entry().with_hash_block(data))?;

        assert_eq!(archive.issue_count(), 0);
    }
    Ok(())
}

#[test]
fn the_last_of_several_hash_blocks_decides() -> TestResult {
    let (_dir, archive) = open(
        entry()
            .with_hash_block([1; 32])
            .with_extra_hash_block(&[2; 32]),
    )?;
    assert_eq!(stated(&archive)?, Some([2; 32]), "the later digest wins");
    assert_eq!(archive.issue_count(), 0);

    let (_dir, archive) = open(
        entry()
            .with_hash_block([1; 32])
            .with_extra_hash_block(&[2; 31]),
    )?;
    assert_eq!(stated(&archive)?, None, "a later malformed block wins too");
    assert_eq!(
        archive.issues(),
        [(0, EntryIssue::HashBlockSize { size: 35 })]
    );

    let (_dir, archive) = open(
        entry()
            .with_hash_block([1; 31])
            .with_extra_hash_block(&[2; 32]),
    )?;
    assert_eq!(stated(&archive)?, Some([2; 32]), "a later valid block wins");
    Ok(())
}
