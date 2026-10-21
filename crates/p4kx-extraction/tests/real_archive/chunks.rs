//! Chunk files of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;

use p4kx_extraction::{Archive, ChunkSource, Entry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const EXTENSIONS: [&str; 10] = [
    ".cgf", ".cga", ".skin", ".chr", ".cgfm", ".cgam", ".skinm", ".chrm", ".caf", ".dba",
];
const SAMPLE_PER_EXTENSION: usize = 40;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_every_chunk_file_extension_opens_and_its_chunks_are_readable() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let mut failures = Vec::new();
    let mut opened = 0;
    for extension in EXTENSIONS {
        let candidates: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| entry.path().to_ascii_lowercase().ends_with(extension))
            .collect();
        let stride = (candidates.len() / SAMPLE_PER_EXTENSION).max(1);
        for entry in candidates
            .into_iter()
            .step_by(stride)
            .take(SAMPLE_PER_EXTENSION)
        {
            match ChunkSource::from_archive(&archive, entry.path()) {
                Ok(source) => {
                    opened += 1;
                    let file = source.file();
                    let readable =
                        (0..file.chunks().len()).all(|index| file.chunk_data(index).is_some());
                    if !readable || file.chunks().is_empty() {
                        failures.push(format!("{}: unreadable or no chunks", entry.path()));
                    }
                }
                Err(error) => failures.push(format!("{}: {error:?}", entry.path())),
            }
        }
    }

    assert!(opened > 0, "the Archive holds chunk files");
    assert!(failures.is_empty(), "failed: {failures:#?}");
    Ok(())
}
