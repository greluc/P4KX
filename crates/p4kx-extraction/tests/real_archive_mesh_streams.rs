//! Survey of the data streams of the mesh chunks of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one. The tables are printed (`--nocapture`).

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
const NORMALS: u32 = 0x9CF3_F615;
const NORMALS_2: u32 = 0x38A5_81FE;
const TANGENTS: u32 = 0xB95E_9A1B;
const QTANGENTS: u32 = 0xEE05_7252;
const BEFORE: [(u32, u32); 4] = [(NORMALS_2, 12), (NORMALS, 4), (QTANGENTS, 8), (TANGENTS, 8)];
const AFTER: [(u32, u32); 7] = [
    (NORMALS_2, 12),
    (NORMALS_2, 4),
    (NORMALS, 12),
    (NORMALS, 4),
    (QTANGENTS, 8),
    (TANGENTS, 8),
    (TANGENTS, 16),
];
const HEADER_LEN: usize = 24 + 48 + 8 + 88;
const SUBMESH_LEN: usize = 48;

#[derive(Default)]
struct Pair {
    streams: u64,
    chunks: u64,
    example: String,
}

#[derive(Default)]
struct Repeat {
    chunks: u64,
    example: String,
}

#[derive(Default)]
struct Survey {
    pairs: BTreeMap<(u32, u32), Pair>,
    repeats: BTreeMap<u32, Repeat>,
    chunks: u64,
    truncated: u64,
    without_normals_before: u64,
    without_normals_after: u64,
    unreadable: Vec<String>,
}

impl Survey {
    fn merge(mut self, other: Self) -> Self {
        for (key, pair) in other.pairs {
            let mine = self.pairs.entry(key).or_default();
            if mine.streams == 0 {
                mine.example = pair.example;
            }
            mine.streams += pair.streams;
            mine.chunks += pair.chunks;
        }
        for (tag, repeat) in other.repeats {
            let mine = self.repeats.entry(tag).or_default();
            if mine.chunks == 0 {
                mine.example = repeat.example;
            }
            mine.chunks += repeat.chunks;
        }
        self.chunks += other.chunks;
        self.truncated += other.truncated;
        self.without_normals_before += other.without_normals_before;
        self.without_normals_after += other.without_normals_after;
        self.unreadable.extend(other.unreadable);
        self
    }
}

fn word(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(bytes).ok()?))
}

fn streams_of(chunk: &[u8]) -> Option<(Vec<(u32, u32)>, bool)> {
    let vertices = usize::try_from(word(chunk, 8)?).ok()?;
    let indices = usize::try_from(word(chunk, 12)?).ok()?;
    let submeshes = usize::try_from(word(chunk, 16)?).ok()?;
    let extra = usize::try_from(word(chunk, 24 + 48 + 4)?).ok()?;
    let mut at = HEADER_LEN
        .checked_add(submeshes.checked_mul(SUBMESH_LEN)?)?
        .checked_add(extra.checked_mul(4)?)?;
    let mut streams = Vec::new();
    while at + 4 <= chunk.len() {
        let tag = word(chunk, at)?;
        at += 4;
        if tag == 0 {
            continue;
        }
        let size = usize::try_from(word(chunk, at)?).ok()?;
        at += 4;
        let count = if tag == INDICES { indices } else { vertices };
        let len = size.checked_mul(count)?;
        let end = at.checked_add(len)?;
        if end > chunk.len() {
            return Some((streams, true));
        }
        streams.push((tag, u32::try_from(size).ok()?));
        at = end.next_multiple_of(8).min(chunk.len());
    }
    Some((streams, false))
}

fn scan(archive: &Archive, entry: &Entry) -> Survey {
    let mut survey = Survey::default();
    if entry.uncompressed_size() > MAX_FILE_LEN {
        return survey;
    }
    let mut bytes = Vec::new();
    if archive
        .open_entry(entry)
        .map_err(|error| error.to_string())
        .and_then(|reader| {
            reader
                .take(MAX_FILE_LEN)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())
        })
        .is_err()
    {
        survey.unreadable.push(entry.path().to_owned());
        return survey;
    }
    let Ok(file) = ChunkFile::parse(bytes) else {
        survey.unreadable.push(entry.path().to_owned());
        return survey;
    };
    for (index, chunk) in file.chunks().iter().enumerate() {
        let kind = chunk.chunk_type();
        if kind != ivo::SKIN_2 && kind != ivo::SKIN {
            continue;
        }
        let Some(data) = file.chunk_data(index) else {
            continue;
        };
        let Some((streams, truncated)) = streams_of(data) else {
            survey.unreadable.push(entry.path().to_owned());
            continue;
        };
        survey.tally(entry.path(), &streams, truncated);
    }
    survey
}

impl Survey {
    fn tally(&mut self, path: &str, streams: &[(u32, u32)], truncated: bool) {
        self.chunks += 1;
        self.truncated += u64::from(truncated);
        self.without_normals_before += u64::from(!streams.iter().any(|pair| BEFORE.contains(pair)));
        self.without_normals_after += u64::from(!streams.iter().any(|pair| AFTER.contains(pair)));
        let mut seen: Vec<(u32, u32)> = Vec::new();
        let mut tags: Vec<u32> = Vec::new();
        let mut repeated: Vec<u32> = Vec::new();
        for &(tag, size) in streams {
            let pair = self.pairs.entry((tag, size)).or_default();
            if pair.streams == 0 {
                path.clone_into(&mut pair.example);
            }
            pair.streams += 1;
            if !seen.contains(&(tag, size)) {
                seen.push((tag, size));
                pair.chunks += 1;
            }
            if tags.contains(&tag) {
                if !repeated.contains(&tag) {
                    repeated.push(tag);
                }
            } else {
                tags.push(tag);
            }
        }
        for tag in repeated {
            let repeat = self.repeats.entry(tag).or_default();
            if repeat.chunks == 0 {
                path.clone_into(&mut repeat.example);
            }
            repeat.chunks += 1;
        }
    }
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey tables are the measurement; the test prints them"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_streams_of_every_mesh_chunk_are_counted_by_tag_and_element_size() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entries: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path();
            EXTENSIONS.iter().any(|extension| {
                path.len() > extension.len()
                    && path[path.len() - extension.len()..].eq_ignore_ascii_case(extension)
            })
        })
        .collect();
    let survey = entries
        .par_iter()
        .map(|entry| scan(&archive, entry))
        .reduce(Survey::default, Survey::merge);

    println!(
        "geometry files: {}, mesh chunks: {}, with an overlong stream: {}, unreadable: {} {:?}",
        entries.len(),
        survey.chunks,
        survey.truncated,
        survey.unreadable.len(),
        survey.unreadable.iter().take(5).collect::<Vec<_>>()
    );
    println!(
        "chunks without a normal stream: {} with the old accepted pairs, {} with the new",
        survey.without_normals_before, survey.without_normals_after
    );
    println!("tag\telement size\tstreams\tchunks\texample");
    for ((tag, size), pair) in &survey.pairs {
        println!(
            "0x{tag:08X}\t{size}\t{}\t{}\t{}",
            pair.streams, pair.chunks, pair.example
        );
    }
    println!("repeated tags: {}", survey.repeats.len());
    for (tag, repeat) in &survey.repeats {
        println!("0x{tag:08X}\t{}\t{}", repeat.chunks, repeat.example);
    }
    assert!(survey.chunks > 0, "the Archive holds mesh chunks");
    Ok(())
}
