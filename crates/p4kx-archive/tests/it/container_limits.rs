// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The limits and checksums of containers through the public API.

use std::error::Error;
use std::path::Path;

use p4kx_archive::{
    Archive, ArchiveError, Checksum, Entry, MAX_CONTAINER_DEPTH, MAX_CONTAINER_ENTRIES,
};
use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const SOCPAK: &str = "Data\\room.socpak";
const MIB: usize = 1024 * 1024;

fn open(dir: &Path, container: Vec<u8>) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(SOCPAK, container))
        .write_to(&path)?;
    Ok(Archive::open(&path)?)
}

fn entry<'a>(archive: &'a Archive, path: &str) -> Result<&'a Entry, Box<dyn Error>> {
    Ok(archive
        .entries()
        .iter()
        .find(|entry| entry.path() == path)
        .ok_or_else(|| format!("no Entry {path}"))?)
}

fn refusal(archive: &Archive, path: &str) -> Result<ArchiveError, Box<dyn Error>> {
    match archive.open_container(entry(archive, path)?) {
        Ok(_) => Err(format!("{path} opened as a container").into()),
        Err(ArchiveError::Container { source, .. }) => Ok(*source),
        Err(other) => Err(format!("{path} failed without naming the Entry: {other:?}").into()),
    }
}

fn position(bytes: &[u8], signature: [u8; 4]) -> Result<usize, Box<dyn Error>> {
    Ok(bytes
        .windows(4)
        .rposition(|window| window == signature)
        .ok_or("signature not found")?)
}

#[test]
fn compressed_containers_of_one_chain_share_one_decoded_size_budget() -> TestResult {
    let dir = tempfile::tempdir()?;
    let declared = 300 * MIB as u64;
    let inner = TestEntry::deflated("inner.socpak", "never decoded")
        .with_payload(vec![0; 200 * 1024])
        .with_uncompressed_size(declared);
    let outer = TestContainer::new().entry(inner).entry(TestEntry::stored(
        "padding.bin",
        vec![0; 300 * MIB - 256 * 1024],
    ));
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::deflated(SOCPAK, outer.build()))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;

    let outer = archive.open_container(entry(&archive, SOCPAK)?)?;
    let refused = refusal(&outer, "inner.socpak")?;

    assert!(
        matches!(&refused, ArchiveError::LimitExceeded { reason, .. } if reason.contains("still decode")),
        "the second level is refused by the budget the first spent, before decoding: {refused:?}"
    );
    Ok(())
}

#[test]
fn entries_whose_data_runs_into_another_entry_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut bytes = TestContainer::new()
        .entry(TestEntry::stored("a.bin", vec![1; 64]))
        .entry(TestEntry::stored("b.bin", vec![2; 64]))
        .build();
    bytes[28..30].copy_from_slice(&40_u16.to_le_bytes());
    let archive = open(dir.path(), bytes)?;

    let refused = refusal(&archive, SOCPAK)?;

    assert!(
        refused.to_string().contains("overlap"),
        "a long local extra field moves the data of a.bin into b.bin: {refused}"
    );
    Ok(())
}

#[test]
fn too_many_entries_are_refused_before_the_directory_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let too_many = (MAX_CONTAINER_ENTRIES as u64 + 1).to_le_bytes();
    let mut zip64 = ArchiveBuilder::new()
        .entry(TestEntry::stored("a.bin", "a"))
        .build();
    let end = position(&zip64, [0x50, 0x4B, 0x06, 0x06])?;
    zip64[end + 32..end + 40].copy_from_slice(&too_many);
    let mut v2 = ArchiveBuilder::v2()
        .entry(TestEntry::stored("a.bin", "a"))
        .build();
    let trailer = v2.len() - 175;
    v2[trailer..trailer + 8].copy_from_slice(&too_many);
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\zip64.socpak", zip64))
        .entry(TestEntry::stored("Data\\v2.socpak", v2))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;

    for container in ["Data\\zip64.socpak", "Data\\v2.socpak"] {
        let refused = refusal(&archive, container)?;
        assert!(
            matches!(&refused, ArchiveError::LimitExceeded { reason, .. } if reason.contains("Entries")),
            "{container}: the count is refused, not the short directory: {refused:?}"
        );
    }
    Ok(())
}

#[test]
fn only_plain_zip_containers_carry_the_crc_32() -> TestResult {
    let dir = tempfile::tempdir()?;
    let zip64 = ArchiveBuilder::new()
        .entry(TestEntry::stored("a.txt", "abc"))
        .build();
    let v2 = ArchiveBuilder::v2()
        .entry(TestEntry::stored("a.txt", "abc"))
        .build();
    let plain = TestContainer::new()
        .entry(TestEntry::stored("a.txt", "abc"))
        .build();
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\zip64.socpak", zip64))
        .entry(TestEntry::stored("Data\\v2.socpak", v2))
        .entry(TestEntry::stored("Data\\plain.socpak", plain))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;

    for (container, expected) in [
        ("Data\\zip64.socpak", Checksum::Crc32c(0x364B_3FB7)),
        ("Data\\v2.socpak", Checksum::Crc32c(0x364B_3FB7)),
        ("Data\\plain.socpak", Checksum::Crc32(0x3524_41C2)),
    ] {
        let opened = archive.open_container(entry(&archive, container)?)?;
        assert_eq!(
            entry(&opened, "a.txt")?.checksum(),
            expected,
            "{container}: CRC-32C or CRC-32 of abc"
        );
    }
    Ok(())
}

#[test]
fn a_container_holding_itself_stops_at_the_depth_limit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut bytes = ArchiveBuilder::v2()
        .entry(TestEntry::stored("self.socpak", vec![0; 16]))
        .build();
    let trailer = bytes.len() - 175;
    let directory = usize::try_from(u64::from_le_bytes(
        bytes[trailer + 16..trailer + 24].try_into()?,
    ))?;
    let whole = (bytes.len() as u64).to_le_bytes();
    bytes[directory + 10..directory + 18].copy_from_slice(&whole);
    bytes[directory + 18..directory + 26].copy_from_slice(&whole);
    let archive = open(dir.path(), bytes)?;

    let mut container = archive.open_container(entry(&archive, SOCPAK)?)?;
    let mut opened = 1;
    let refused = loop {
        match container.open_container(entry(&container, "self.socpak")?) {
            Ok(inner) => {
                container = inner;
                opened += 1;
            }
            Err(ArchiveError::Container { source, .. }) => break *source,
            Err(other) => return Err(other.into()),
        }
    };

    assert_eq!(
        opened, MAX_CONTAINER_DEPTH,
        "the same container opens 8 times"
    );
    assert!(
        matches!(&refused, ArchiveError::LimitExceeded { reason, .. } if reason.contains("deeper")),
        "then the depth limit stops it: {refused:?}"
    );
    Ok(())
}
