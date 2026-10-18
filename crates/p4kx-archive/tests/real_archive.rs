//! Reading Entries of a real Archive; runs only when `P4KX_DATA_P4K` names one.

use std::collections::HashMap;
use std::error::Error;
use std::io;

use p4kx_archive::{Archive, Compression};

const PER_KIND: usize = 25;
const MAX_SIZE: u64 = 64 << 20;

#[test]
fn reads_entries_of_every_kind_from_a_real_archive() -> Result<(), Box<dyn Error>> {
    let Some(path) = std::env::var_os("P4KX_DATA_P4K") else {
        return Ok(());
    };
    let archive = Archive::open(path)?;
    let mut read_per_kind: HashMap<(Compression, bool), usize> = HashMap::new();
    for entry in archive.entries() {
        let kind = (entry.compression(), entry.is_encrypted());
        let count = read_per_kind.entry(kind).or_default();
        if *count >= PER_KIND || entry.uncompressed_size() > MAX_SIZE {
            continue;
        }
        *count += 1;
        let copied = io::copy(&mut archive.open_entry(entry)?, &mut io::sink())
            .map_err(|error| format!("{}: {error}", entry.path()))?;
        assert_eq!(
            copied,
            entry.uncompressed_size(),
            "{} yields its uncompressed size",
            entry.path()
        );
    }
    assert!(
        read_per_kind.keys().any(|(_, encrypted)| *encrypted),
        "the Archive has encrypted Entries: {read_per_kind:?}"
    );
    Ok(())
}
