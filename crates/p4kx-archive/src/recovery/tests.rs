// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::time::{Duration, Instant};

use super::{CHUNK_LEN, scan};
use crate::source::Source;

const NAME_LEN: usize = 100;

fn local(out: &mut Vec<u8>, name: &[u8]) {
    out.extend_from_slice(&0x0403_4B50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 22]);
    out.extend_from_slice(&u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(name);
}

fn central(out: &mut Vec<u8>, name: &[u8], offset: u32) {
    out.extend_from_slice(&0x0201_4B50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    out.extend_from_slice(&u16::try_from(name.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&[0; 12]);
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(name);
}

fn names(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|index| {
            let mut name = format!("Data\\file{index:08}").into_bytes();
            name.resize(NAME_LEN, b'x');
            name
        })
        .collect()
}

#[test]
fn a_directory_larger_than_one_block_is_read_in_few_reads() {
    let names = names(100_000);
    let mut bytes = Vec::new();
    let offsets: Vec<u32> = names
        .iter()
        .map(|name| {
            let offset = u32::try_from(bytes.len()).unwrap_or(0);
            local(&mut bytes, name);
            offset
        })
        .collect();
    let directory = bytes.len();
    for (name, offset) in names.iter().zip(offsets) {
        central(&mut bytes, name, offset);
    }
    assert!(
        bytes.len() - directory > 2 * CHUNK_LEN,
        "directory spans blocks"
    );
    let total = bytes.len();
    let source = Source::memory(bytes);

    let started = Instant::now();
    let found = scan(&source, 0).expect("scan");

    assert_eq!(found.entries.len(), 100_000);
    assert!(!found.cut);
    assert!(
        found.reads <= (total / CHUNK_LEN) as u64 * 2 + 4,
        "{} reads for {total} bytes",
        found.reads
    );
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn dense_overlapping_signatures_stop_at_the_work_budget() {
    let bytes = 0x0201_4B50_u32.to_le_bytes().repeat(2 << 20);
    let source = Source::memory(bytes);

    let started = Instant::now();
    let found = scan(&source, 0).expect("scan");

    assert_eq!(found.entries.len(), 0);
    assert!(found.cut, "the budget ends the scan");
    assert!(started.elapsed() < Duration::from_secs(20));
}
