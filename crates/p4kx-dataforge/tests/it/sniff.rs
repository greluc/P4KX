// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Recognising a DataForge file by its header.

use p4kx_dataforge::{SNIFF_LEN, looks_like_dataforge};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct};

fn sniff(bytes: &[u8]) -> bool {
    looks_like_dataforge(bytes, bytes.len() as u64)
}

fn padded(head: &[u8], len: usize) -> Vec<u8> {
    let mut bytes = head.to_vec();
    bytes.resize(len, 0x41);
    bytes
}

#[test]
fn a_dataforge_of_any_version_is_recognised() {
    let database = |version| {
        DataForgeBuilder::new()
            .version(version)
            .structure(TestStruct::new("Item"))
            .record(TestRecord::new(1, "Item", "Item.A", "a.xml"))
            .build()
    };

    for version in [4, 5, 6, 7, 8] {
        assert!(sniff(&database(version)), "version {version}");
    }
}

#[test]
fn a_header_alone_with_no_tables_is_recognised() {
    assert!(sniff(&DataForgeBuilder::new().build()));
}

#[test]
fn zero_filled_files_are_not() {
    assert!(!sniff(&vec![0; 4096]));
    assert!(!sniff(&[0; SNIFF_LEN]));
}

#[test]
fn an_executable_is_not() {
    let head = [
        0x4d, 0x5a, 0x90, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    ];
    assert!(!sniff(&padded(&head, 1 << 20)));
}

#[test]
fn gzip_and_an_archive_are_not() {
    assert!(!sniff(&padded(
        &[0x1f, 0x8b, 0x08, 0x00, 0x12, 0x34, 0x56, 0x78],
        4096
    )));
    assert!(!sniff(&padded(b"PK\x03\x04\x14\x00\x00\x00", 4096)));
}

#[test]
fn counts_larger_than_the_file_pass_only_for_a_supported_version() {
    let claiming = |version| {
        let mut bytes = DataForgeBuilder::new().version(version).build();
        bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        bytes
    };

    assert!(sniff(&claiming(6)), "a truncated version 6");
    assert!(sniff(&claiming(8)), "a truncated version 8");
    assert!(!sniff(&claiming(5)), "four billion structs do not fit");
    assert!(!sniff(&claiming(3)), "four billion structs do not fit");
}

#[test]
fn a_file_shorter_than_the_header_is_not() {
    let bytes = DataForgeBuilder::new().build();

    assert!(!sniff(&bytes[..SNIFF_LEN - 1]));
    assert!(!sniff(&[]));
}
