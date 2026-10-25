// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Survey of the `CrCh` chunk files of a real Archive, read without the Archive key: Entries that
//! are encrypted are counted and left out. Runs with `--ignored` when `P4KX_DATA_P4K` names an
//! Archive; the table is printed with `--nocapture`.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, Container};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const MAX_FILE_LEN: u64 = 256 << 20;

#[derive(Default)]
struct Stat {
    files: u64,
    chunks: u64,
    extensions: BTreeMap<String, u64>,
    example: String,
}

#[derive(Default)]
struct Survey {
    encrypted: BTreeMap<String, u64>,
    signatures: BTreeMap<String, u64>,
    crch: BTreeMap<(u32, u32, u8), Stat>,
    failed: Vec<String>,
}

impl Survey {
    fn merge(mut self, other: Self) -> Self {
        for (key, count) in other.encrypted {
            *self.encrypted.entry(key).or_default() += count;
        }
        for (key, count) in other.signatures {
            *self.signatures.entry(key).or_default() += count;
        }
        for (key, stat) in other.crch {
            let mine = self.crch.entry(key).or_default();
            if mine.files == 0 {
                mine.example = stat.example;
            }
            mine.files += stat.files;
            mine.chunks += stat.chunks;
            for (extension, count) in stat.extensions {
                *mine.extensions.entry(extension).or_default() += count;
            }
        }
        self.failed.extend(other.failed);
        self
    }
}

fn extension_of(entry: &Entry) -> String {
    entry
        .path()
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn printable(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_graphic() { c } else { '?' })
        .collect()
}

fn scan(archive: &Archive, entry: &Entry, survey: &mut Survey) {
    let extension = extension_of(entry);
    if entry.is_encrypted() {
        *survey.encrypted.entry(extension).or_default() += 1;
        return;
    }
    let mut head = [0_u8; 12];
    let Ok(mut reader) = archive.open_entry(entry) else {
        return;
    };
    if reader.read_exact(&mut head).is_err() {
        return;
    }
    let Some((signature, version)) = signature_of(&head) else {
        return;
    };
    *survey
        .signatures
        .entry(format!("{signature} 0x{version:X} .{extension}"))
        .or_default() += 1;
    if signature != "CrCh" || entry.uncompressed_size() > MAX_FILE_LEN {
        return;
    }
    let mut bytes = head.to_vec();
    if reader.take(MAX_FILE_LEN).read_to_end(&mut bytes).is_err() {
        return;
    }
    match ChunkFile::parse(bytes) {
        Ok(file) if file.container() == Container::CrCh => {
            count_chunks(survey, entry, &extension, &file);
        }
        Ok(_) => {}
        Err(error) => survey
            .failed
            .push(format!("{}: {error}", printable(entry.path()))),
    }
}

fn signature_of(head: &[u8; 12]) -> Option<(&'static str, u32)> {
    let word = |at: usize| {
        head.get(at..at + 4)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map(u32::from_le_bytes)
    };
    if head.starts_with(b"CryTek") {
        Some(("CryTek", word(8)?))
    } else if head.starts_with(b"CrCh") {
        Some(("CrCh", word(4)?))
    } else if head.starts_with(b"#ivo") {
        Some(("#ivo", word(4)?))
    } else {
        None
    }
}

fn count_chunks(survey: &mut Survey, entry: &Entry, extension: &str, file: &ChunkFile<Vec<u8>>) {
    let mut seen = Vec::new();
    for chunk in file.chunks() {
        let key = (
            chunk.chunk_type().id(),
            chunk.version(),
            u8::from(chunk.byte_order() == p4kx_chunks::ByteOrder::Big),
        );
        let stat = survey.crch.entry(key).or_default();
        if stat.chunks == 0 {
            stat.example = printable(entry.path());
        }
        stat.chunks += 1;
        *stat.extensions.entry(extension.to_owned()).or_default() += 1;
        if !seen.contains(&key) {
            seen.push(key);
            stat.files += 1;
        }
    }
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey table is the measurement; the test prints it"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_crch_files_of_the_archive_are_counted_by_container_version_and_chunk() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let survey = archive
        .entries()
        .par_iter()
        .fold(Survey::default, |mut survey, entry| {
            scan(&archive, entry, &mut survey);
            survey
        })
        .reduce(Survey::default, Survey::merge);

    println!("encrypted Entries left out, by extension:");
    for (extension, count) in &survey.encrypted {
        println!("  .{extension}: {count}");
    }
    println!("chunk file signatures of the unencrypted Entries:");
    for (key, count) in &survey.signatures {
        println!("  {key}: {count}");
    }
    println!("CrCh chunks (type, version, big-endian): files, chunks, extensions, example");
    for ((id, version, big), stat) in &survey.crch {
        let name = p4kx_chunks::ChunkType::new(Container::CrCh, *id)
            .name()
            .unwrap_or("UNKNOWN");
        println!(
            "  0x{id:04X} {name} v0x{version:X} big={big}: {} files, {} chunks, {:?}, {}",
            stat.files, stat.chunks, stat.extensions, stat.example
        );
    }
    println!(
        "CrCh files that failed to parse: {} {:?}",
        survey.failed.len(),
        survey.failed.iter().take(10).collect::<Vec<_>>()
    );
    Ok(())
}
