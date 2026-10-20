//! Opening the socpak containers of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K`
//! names one.

use std::error::Error;
use std::io::{self, Write};

use p4kx_archive::{Archive, Checksum, Entry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLED: usize = 200;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn socpak_containers_open_and_their_entries_match_their_crc_32() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let socpaks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| is_socpak(entry))
        .collect();
    let step = (socpaks.len() / SAMPLED).max(1);

    let mut opened = 0;
    let mut read = 0;
    for entry in socpaks.iter().step_by(step) {
        let (containers, entries) = verify(&archive, entry)?;
        opened += containers;
        read += entries;
    }

    assert!(
        opened >= SAMPLED.min(socpaks.len()),
        "opened {opened} containers"
    );
    assert!(read > opened, "read {read} Entries inside them");
    Ok(())
}

fn verify(parent: &Archive, entry: &Entry) -> Result<(usize, usize), Box<dyn Error>> {
    let container = parent.open_container(entry)?;
    let (mut containers, mut entries) = (1, 0);
    for inner in container.entries() {
        let mut sink = Crc32Sink::default();
        let size = io::copy(&mut container.open_entry(inner)?, &mut sink)?;
        let location = container.location().display();
        assert_eq!(
            size,
            inner.uncompressed_size(),
            "{location}: {}",
            inner.path()
        );
        assert_eq!(
            Checksum::Crc32(sink.hasher.finalize()),
            inner.checksum(),
            "{location}: {}",
            inner.path()
        );
        entries += 1;
        if is_socpak(inner) {
            let (nested_containers, nested_entries) = verify(&container, inner)?;
            containers += nested_containers;
            entries += nested_entries;
        }
    }
    Ok((containers, entries))
}

fn is_socpak(entry: &Entry) -> bool {
    entry
        .path()
        .rsplit('.')
        .next()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("socpak"))
}

#[derive(Default)]
struct Crc32Sink {
    hasher: crc32fast::Hasher,
}

impl Write for Crc32Sink {
    fn write(&mut self, chunk: &[u8]) -> io::Result<usize> {
        self.hasher.update(chunk);
        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
