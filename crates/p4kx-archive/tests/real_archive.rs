//! Verification against a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use p4kx_archive::{Archive, Build, Compression, Entry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const OPEN_LIMIT: Duration = Duration::from_secs(10);
const PER_KIND: usize = 25;
const MAX_SIZE: u64 = 64 << 20;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn opens_and_lists_a_real_archive_in_seconds() -> TestResult {
    let path = real_archive_path()?;
    let started = Instant::now();
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::real_archive_key()?))?;
    let listed = archive
        .entries()
        .iter()
        .filter(|entry| !entry.path().is_empty())
        .count();
    let elapsed = started.elapsed();

    assert!(!archive.entries().is_empty(), "the Archive has Entries");
    assert_eq!(listed, archive.entries().len(), "every Entry has a path");
    assert!(
        elapsed < OPEN_LIMIT,
        "opening and listing {listed} Entries took {elapsed:?}, more than {OPEN_LIMIT:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn reads_the_build_of_a_real_archive() -> TestResult {
    let path = real_archive_path()?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::real_archive_key()?))?;
    let folder = path.parent().ok_or("the Archive path has a folder")?;

    let manifest_build = Build::read_from(folder)?
        .ok_or("the Channel folder of the Archive has a build_manifest.id")?;

    let build = archive.build().ok_or("the Archive has a Build")?;
    assert_eq!(
        build, &manifest_build,
        "the Archive carries the Build of the manifest next to it"
    );
    assert!(!build.version().is_empty(), "the Build has a version");
    assert!(!build.build_id().is_empty(), "the Build has a BuildId");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn reads_entries_of_every_compression_and_encryption_state() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;

    let samples = samples_per_kind(archive.entries());
    for (kind, entries) in &samples {
        for entry in entries {
            let (size, crc) = read_fully(&archive, entry)
                .map_err(|error| format!("{kind:?} {}: {error}", entry.path()))?;
            assert_eq!(
                size,
                entry.uncompressed_size(),
                "{} yields its uncompressed size",
                entry.path()
            );
            assert_eq!(
                Some(crc),
                entry.crc32c(),
                "{} matches its CRC-32C",
                entry.path()
            );
        }
    }

    assert!(
        samples.keys().any(|(_, encrypted)| *encrypted),
        "the Archive has encrypted Entries: {:?}",
        samples.keys()
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn verified_reads_of_every_compression_and_encryption_state_find_no_mismatch() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;

    let samples = samples_per_kind(archive.entries());
    let mut checked = 0;
    for entries in samples.values() {
        for entry in entries {
            let size = io::copy(&mut archive.open_entry_verified(entry)?, &mut io::sink())
                .map_err(|error| format!("{}: {error}", entry.path()))?;
            assert_eq!(
                size,
                entry.uncompressed_size(),
                "{} reads fully",
                entry.path()
            );
            checked += 1;
        }
    }

    assert!(checked > 0, "the Archive yields samples");
    Ok(())
}

type Kind = (Compression, bool);

fn samples_per_kind(entries: &[Entry]) -> HashMap<Kind, Vec<&Entry>> {
    let mut samples: HashMap<Kind, Vec<&Entry>> = HashMap::new();
    let mut smallest: HashMap<Kind, &Entry> = HashMap::new();
    for entry in entries {
        let kind = (entry.compression(), entry.is_encrypted());
        let picked = samples.entry(kind).or_default();
        if picked.len() < PER_KIND && entry.uncompressed_size() <= MAX_SIZE {
            picked.push(entry);
        }
        smallest
            .entry(kind)
            .and_modify(|small| {
                if entry.uncompressed_size() < small.uncompressed_size() {
                    *small = entry;
                }
            })
            .or_insert(entry);
    }
    for (kind, entry) in smallest {
        let picked = samples.entry(kind).or_default();
        if picked.is_empty() {
            picked.push(entry);
        }
    }
    samples
}

fn read_fully(archive: &Archive, entry: &Entry) -> Result<(u64, u32), Box<dyn Error>> {
    let mut sink = CrcSink::default();
    let size = io::copy(&mut archive.open_entry(entry)?, &mut sink)?;
    Ok((size, sink.crc))
}

#[derive(Default)]
struct CrcSink {
    crc: u32,
}

impl Write for CrcSink {
    fn write(&mut self, chunk: &[u8]) -> io::Result<usize> {
        self.crc = crc32c::crc32c_append(self.crc, chunk);
        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
