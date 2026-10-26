// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The palette index of the object records sits at the same offset in every record layout.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::sync::Mutex;

use p4kx_chunks::{ChunkFile, ChunkType, Container};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const PALETTE_AT: usize = 172;
const NAME_LEN: usize = 256;
const NO_PALETTE: usize = 255;

#[derive(Default, Debug)]
struct Counts {
    records: usize,
    in_range: usize,
    zero_or_none: usize,
    other_out_of_range: usize,
}

fn le32(bytes: &[u8], at: usize) -> Option<usize> {
    let value = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
    usize::try_from(value).ok()
}

fn count(data: &[u8], tally: &Mutex<BTreeMap<usize, Counts>>) -> Option<()> {
    let models = le32(data, 4)?;
    let mut at = 8 + models * NAME_LEN;
    let materials = usize::from(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
    let palettes = usize::from(u16::from_le_bytes(
        data.get(at + 2..at + 4)?.try_into().ok()?,
    ));
    at += 4 + (materials + palettes) * NAME_LEN;
    let layout = le32(data, at)?;
    let stride = match layout {
        8..=10 => 176,
        12 => 184,
        13 => 192,
        14 => 200,
        15 => 208,
        _ => return None,
    };
    let declared = le32(data, at + 28)?;
    let mut table = data.get(at + 32..at + 32 + declared)?;
    let mut counts = Counts::default();
    while let Some(kind) = le32(table, 0) {
        let length = match kind {
            1 => stride,
            16 => 136,
            _ => break,
        };
        let (record, rest) = table.split_at_checked(length)?;
        table = rest;
        if kind == 1 {
            let index = usize::from(*record.get(PALETTE_AT)?);
            counts.records += 1;
            if index < palettes {
                counts.in_range += 1;
            } else if index == 0 || index == NO_PALETTE {
                counts.zero_or_none += 1;
            } else {
                counts.other_out_of_range += 1;
            }
        }
    }
    let mut tally = tally.lock().ok()?;
    let total = tally.entry(layout).or_default();
    total.records += counts.records;
    total.in_range += counts.in_range;
    total.zero_or_none += counts.zero_or_none;
    total.other_out_of_range += counts.other_out_of_range;
    drop(tally);
    Some(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts go into the PR body")]
fn the_palette_index_is_in_range_zero_or_255_in_every_layout() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let tally = Mutex::new(BTreeMap::new());
    let socpaks: Vec<_> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".socpak"))
        .collect();
    let chunk_type = ChunkType::new(Container::CrCh, 0x0010);
    socpaks.par_iter().for_each(|entry| {
        let Ok(root) = archive.open_container(entry) else {
            return;
        };
        for inner in root.entries() {
            if !inner.path().to_ascii_lowercase().ends_with(".soc") {
                continue;
            }
            let mut bytes = Vec::new();
            let Ok(mut reader) = root.open_entry(inner) else {
                continue;
            };
            if reader.read_to_end(&mut bytes).is_err() {
                continue;
            }
            let Ok(file) = ChunkFile::parse(&bytes) else {
                continue;
            };
            for (index, _) in file.chunks_of(chunk_type) {
                if let Some(data) = file.chunk_data(index) {
                    count(data, &tally);
                }
            }
        }
    });
    let tally = tally.into_inner()?;
    println!("{tally:#?}");
    for (layout, counts) in &tally {
        assert_eq!(counts.other_out_of_range, 0, "layout {layout}: {counts:?}");
    }
    Ok(())
}
