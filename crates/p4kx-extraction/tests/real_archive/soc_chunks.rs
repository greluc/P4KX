// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Surveys the chunk types of every `.soc` file of a real Archive, nested socpaks included: each
//! type found is named in the `CrCh` table, the text chunks read, and the counts are printed.

#![expect(clippy::print_stdout, reason = "the survey is the output")]

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::sync::Mutex;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, Container, crch};
use p4kx_cryxml as cryxml;
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const MAX_SOC_LEN: u64 = 128 << 20;
const MAX_DEPTH: usize = 4;

#[derive(Default)]
struct Tally {
    files: usize,
    unreadable: usize,
    skipped: usize,
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
            let data = file.chunk_data(index);
            let readable = if kind == crch::CRY_XML_B || kind == crch::CRY_XML_B_2 {
                data.is_some_and(|data| cryxml::read(data).is_ok())
            } else if kind == crch::MODEL_PATHS || kind == crch::JSON {
                data.is_some_and(|data| std::str::from_utf8(data).is_ok())
            } else {
                true
            };
            if !readable {
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
        self.skipped += other.skipped;
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
        if is_ending(entry, ".socpak") {
            if depth < MAX_DEPTH {
                nested.push(entry);
            } else {
                local.skipped += 1;
            }
            continue;
        }
        if !is_ending(entry, ".soc") {
            continue;
        }
        if entry.uncompressed_size() > MAX_SOC_LEN {
            local.skipped += 1;
            continue;
        }
        bytes.clear();
        let read = container
            .open_entry(entry)
            .ok()
            .and_then(|reader| reader.take(MAX_SOC_LEN).read_to_end(&mut bytes).ok());
        match read.and_then(|_| ChunkFile::parse(bytes.as_slice()).ok()) {
            Some(file) => local.add_file(&file),
            None => local.unreadable += 1,
        }
    }
    tally
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .merge(local);
    nested
        .par_iter()
        .for_each(|entry| match container.open_container(entry) {
            Ok(inner) => survey(&inner, depth + 1, tally),
            Err(_) => {
                tally
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .skipped += 1;
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
    socpaks
        .par_iter()
        .for_each(|entry| match archive.open_container(entry) {
            Ok(container) => survey(&container, 0, &tally),
            Err(_) => {
                tally
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .skipped += 1;
            }
        });
    let tally = tally
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    println!(
        "{} socpaks, {} .soc files, {} unreadable, {} skipped",
        socpaks.len(),
        tally.files,
        tally.unreadable,
        tally.skipped
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
    assert_eq!(tally.unreadable, 0, "every .soc file reads");
    assert_eq!(tally.skipped, 0, "no container or file is skipped");
    assert_eq!(
        tally.bad_text, 0,
        "every CryXmlB, Json and ModelPaths chunk reads as text"
    );
    assert!(
        unnamed.is_empty(),
        "chunk types missing from the CrCh table: {unnamed:04X?}"
    );
    Ok(())
}
