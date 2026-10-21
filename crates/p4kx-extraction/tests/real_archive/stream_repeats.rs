//! Survey of repeated stream tags in the mesh chunks of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one, prints the tags that occur twice in a chunk with an example, and
//! fails when a tag the model reads repeats, a stream overruns its chunk or a file cannot be read.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_testkit::{mesh_streams, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const EXTENSIONS: [&str; 4] = ["cgfm", "cgam", "skinm", "chrm"];
const MAX_FILE_LEN: u64 = 512 << 20;
const READ_TAGS: [u32; 7] = [
    0xEECD_C168,
    0x9132_9AE9,
    0xB3A7_0D5E,
    0x9CF3_F615,
    0x38A5_81FE,
    0xB95E_9A1B,
    0xEE05_7252,
];

#[derive(Default)]
struct Survey {
    chunks: u64,
    overlong: u64,
    unreadable: Vec<String>,
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
        self.unreadable.extend(other.unreadable);
        self
    }
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
        survey.unreadable.push(entry.path().to_owned());
        return survey;
    };
    for (index, chunk) in file.chunks().iter().enumerate() {
        let kind = chunk.chunk_type();
        if kind != ivo::SKIN_2 && kind != ivo::SKIN {
            continue;
        }
        let Some(walked) = file.chunk_data(index).and_then(mesh_streams) else {
            survey.unreadable.push(entry.path().to_owned());
            continue;
        };
        survey.chunks += 1;
        survey.overlong += u64::from(walked.truncated);
        let mut seen: Vec<u32> = Vec::new();
        for stream in &walked.streams {
            if seen.contains(&stream.tag) {
                let repeat = survey.repeats.entry(stream.tag).or_default();
                if repeat.0 == 0 {
                    entry.path().clone_into(&mut repeat.1);
                }
                repeat.0 += 1;
            } else {
                seen.push(stream.tag);
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
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let survey = archive
        .entries()
        .par_iter()
        .filter(|entry| {
            let path = entry.path();
            EXTENSIONS.iter().any(|extension| {
                path.len() > extension.len()
                    && path[path.len() - extension.len()..].eq_ignore_ascii_case(extension)
            })
        })
        .map(|entry| scan(&archive, entry))
        .reduce(Survey::default, Survey::merge);
    println!(
        "mesh chunks: {}, with an overlong stream: {}, unreadable: {} {:?}",
        survey.chunks,
        survey.overlong,
        survey.unreadable.len(),
        survey.unreadable.iter().take(5).collect::<Vec<_>>()
    );
    println!("tags repeated within a chunk: {}", survey.repeats.len());
    for (tag, (chunks, example)) in &survey.repeats {
        println!("0x{tag:08X}\t{chunks} chunks\t{example}");
    }
    assert!(survey.chunks > 0, "the Archive holds mesh chunks");
    let read_repeats: Vec<_> = survey
        .repeats
        .iter()
        .filter(|(tag, _)| READ_TAGS.contains(tag))
        .collect();
    assert!(
        read_repeats.is_empty(),
        "a stream the model reads repeats: {read_repeats:#?}"
    );
    assert_eq!(survey.overlong, 0, "streams that overrun their chunk");
    assert!(
        survey.unreadable.is_empty(),
        "unreadable: {:#?}",
        survey.unreadable
    );
    Ok(())
}
