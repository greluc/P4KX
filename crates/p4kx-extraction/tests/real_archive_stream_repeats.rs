//! Survey of repeated stream tags in the mesh chunks of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one, and prints the tags that occur twice in a chunk with an example.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const EXTENSIONS: [&str; 4] = ["cgfm", "cgam", "skinm", "chrm"];
const MAX_FILE_LEN: u64 = 512 << 20;
const INDICES: u32 = 0xEECD_C168;
const HEADER_LEN: usize = 24 + 48 + 8 + 88;
const SUBMESH_LEN: usize = 48;

#[derive(Default)]
struct Survey {
    chunks: u64,
    overlong: u64,
    unreadable: u64,
    repeats: BTreeMap<u32, (u64, String)>,
}

impl Survey {
    fn merge(mut self, other: Self) -> Self {
        for (tag, (chunks, example)) in other.repeats {
            let mine = self.repeats.entry(tag).or_default();
            if mine.0 == 0 {
                mine.1 = example;
            }
            mine.0 += chunks;
        }
        self.chunks += other.chunks;
        self.overlong += other.overlong;
        self.unreadable += other.unreadable;
        self
    }
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(bytes).ok()?))
}

fn tags_of(chunk: &[u8]) -> Option<(Vec<u32>, bool)> {
    let vertices = usize::try_from(word(chunk, 8)?).ok()?;
    let indices = usize::try_from(word(chunk, 12)?).ok()?;
    let submeshes = usize::try_from(word(chunk, 16)?).ok()?;
    let extra = usize::try_from(word(chunk, 24 + 48 + 4)?).ok()?;
    let mut at = HEADER_LEN
        .checked_add(submeshes.checked_mul(SUBMESH_LEN)?)?
        .checked_add(extra.checked_mul(4)?)?;
    let mut tags = Vec::new();
    while at + 4 <= chunk.len() {
        let tag = word(chunk, at)?;
        at += 4;
        if tag == 0 {
            continue;
        }
        let size = usize::try_from(word(chunk, at)?).ok()?;
        at += 4;
        let count = if tag == INDICES { indices } else { vertices };
        let end = at.checked_add(size.checked_mul(count)?)?;
        if end > chunk.len() {
            return Some((tags, true));
        }
        tags.push(tag);
        at = end.next_multiple_of(8).min(chunk.len());
    }
    Some((tags, false))
}

fn scan(archive: &Archive, entry: &Entry) -> Survey {
    let mut survey = Survey::default();
    let mut bytes = Vec::new();
    let read = archive
        .open_entry(entry)
        .map_err(|error| error.to_string())
        .and_then(|reader| {
            reader
                .take(MAX_FILE_LEN)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())
        });
    let Ok(file) = read
        .map_err(|_| ())
        .and_then(|_| ChunkFile::parse(bytes).map_err(|_| ()))
    else {
        survey.unreadable = 1;
        return survey;
    };
    for (index, chunk) in file.chunks().iter().enumerate() {
        let kind = chunk.chunk_type();
        if kind != ivo::SKIN_2 && kind != ivo::SKIN {
            continue;
        }
        let Some((tags, overlong)) = file.chunk_data(index).and_then(tags_of) else {
            survey.unreadable += 1;
            continue;
        };
        survey.chunks += 1;
        survey.overlong += u64::from(overlong);
        let mut seen: Vec<u32> = Vec::new();
        for tag in tags {
            if seen.contains(&tag) {
                let repeat = survey.repeats.entry(tag).or_default();
                if repeat.0 == 0 {
                    entry.path().clone_into(&mut repeat.1);
                }
                repeat.0 += 1;
            } else {
                seen.push(tag);
            }
        }
    }
    survey
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey table is the measurement; the test prints it"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn repeated_stream_tags_in_mesh_chunks_are_counted_per_tag() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let survey = archive
        .entries()
        .par_iter()
        .filter(|entry| {
            let path = entry.path();
            entry.uncompressed_size() <= MAX_FILE_LEN
                && EXTENSIONS.iter().any(|extension| {
                    path.len() > extension.len()
                        && path[path.len() - extension.len()..].eq_ignore_ascii_case(extension)
                })
        })
        .map(|entry| scan(&archive, entry))
        .reduce(Survey::default, Survey::merge);
    println!(
        "mesh chunks: {}, with an overlong stream: {}, unreadable: {}",
        survey.chunks, survey.overlong, survey.unreadable
    );
    println!("tags repeated within a chunk: {}", survey.repeats.len());
    for (tag, (chunks, example)) in &survey.repeats {
        println!("0x{tag:08X}\t{chunks} chunks\t{example}");
    }
    assert!(survey.chunks > 0, "the Archive holds mesh chunks");
    Ok(())
}
