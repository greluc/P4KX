// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Surveys the chunk types of every `.soc` file of a real Archive, nested socpaks included: each
//! type found is named in the `CrCh` table, and the counts are printed.

#![expect(clippy::print_stdout, reason = "the survey is the output")]

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::sync::Mutex;

use p4kx_extraction::chunks::{ChunkFile, Container};
use p4kx_extraction::cryxml;
use p4kx_extraction::{Archive, Entry};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const MAX_SOC_LEN: u64 = 128 << 20;
const MAX_DEPTH: usize = 4;

#[derive(Default)]
struct Tally {
    files: usize,
    unreadable: usize,
    bad_text: usize,
    chunks: BTreeMap<u32, TypeCount>,
}

#[derive(Default)]
struct TypeCount {
    name: Option<&'static str>,
    chunks: usize,
    files: usize,
    versions: BTreeMap<u32, usize>,
}

fn is_ending(entry: &Entry, extension: &str) -> bool {
    entry.path().to_ascii_lowercase().ends_with(extension)
}
impl Tally {
    fn add_file(&mut self, file: &ChunkFile<&[u8]>) {
        self.files += 1;
        let mut seen = Vec::new();
        for (index, chunk) in file.chunks().iter().enumerate() {
            let kind = chunk.chunk_type();
            if kind.container() != Container::CrCh {
                continue;
            }
            if matches!(kind.id(), 0x0004 | 0x0014)
                && file
                    .chunk_data(index)
                    .is_none_or(|data| cryxml::read(data).is_err())
            {
                self.bad_text += 1;
            }
            let count = self.chunks.entry(kind.id()).or_default();
            count.name = kind.name();
            count.chunks += 1;
            *count.versions.entry(chunk.version()).or_default() += 1;
            if !seen.contains(&kind.id()) {
                seen.push(kind.id());
                count.files += 1;
            }
        }
    }

    fn merge(&mut self, other: Self) {
        self.files += other.files;
        self.unreadable += other.unreadable;
        self.bad_text += other.bad_text;
        for (id, count) in other.chunks {
            let sum = self.chunks.entry(id).or_default();
            sum.name = count.name;
            sum.chunks += count.chunks;
            sum.files += count.files;
            for (version, n) in count.versions {
                *sum.versions.entry(version).or_default() += n;
            }
        }
    }
}

fn survey(container: &Archive, depth: usize, tally: &Mutex<Tally>) {
    let mut local = Tally::default();
    let mut nested = Vec::new();
    let mut bytes = Vec::new();
    for entry in container.entries() {
        if is_ending(entry, ".socpak") && depth < MAX_DEPTH {
            nested.push(entry);
            continue;
        }
        if !is_ending(entry, ".soc") || entry.uncompressed_size() > MAX_SOC_LEN {
            continue;
        }
        bytes.clear();
        let read = container
            .open_entry(entry)
            .ok()
            .and_then(|mut reader| reader.read_to_end(&mut bytes).ok());
        match read.and_then(|_| ChunkFile::parse(bytes.as_slice()).ok()) {
            Some(file) => local.add_file(&file),
            None => local.unreadable += 1,
        }
    }
    tally
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .merge(local);
    nested.par_iter().for_each(|entry| {
        if let Ok(inner) = container.open_container(entry) {
            survey(&inner, depth + 1, tally);
        }
    });
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_chunk_type_of_the_soc_files_is_named() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let tally = Mutex::new(Tally::default());
    let socpaks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| is_ending(entry, ".socpak"))
        .collect();
    socpaks.par_iter().for_each(|entry| {
        if let Ok(container) = archive.open_container(entry) {
            survey(&container, 0, &tally);
        }
    });
    let tally = tally
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    println!(
        "{} socpaks, {} .soc files, {} unreadable",
        socpaks.len(),
        tally.files,
        tally.unreadable
    );
    println!(
        "{:>8}  {:<20}  {:>9}  {:>8}  versions",
        "Type ID", "Name", "Chunks", "Files"
    );
    let mut unnamed = Vec::new();
    for (id, count) in &tally.chunks {
        println!(
            "0x{id:04X}    {:<20}  {:>9}  {:>8}  {:?}",
            count.name.unwrap_or("unknown"),
            count.chunks,
            count.files,
            count.versions
        );
        if count.name.is_none() {
            unnamed.push(*id);
        }
    }
    assert!(tally.files > 0, "the Archive has .soc files");
    assert_eq!(tally.bad_text, 0, "every CryXmlB chunk reads as XML");
    assert!(
        unnamed.is_empty(),
        "chunk types missing from the CrCh table: {unnamed:04X?}"
    );
    Ok(())
}
