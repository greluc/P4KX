use std::io::{self, BufWriter, Write};
use std::path::Path;

use p4kx_extraction::{Build, Entry};
use serde::Serialize;

/// Writes `value` as one line of compact JSON to stdout.
pub(crate) fn print(value: &impl Serialize) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    serde_json::to_writer(&mut out, value)?;
    writeln!(out)?;
    out.flush()
}

/// Returns a path as JSON text.
pub(crate) fn path(path: &Path) -> String {
    path.display().to_string()
}

/// The JSON shape of a Build.
#[derive(Serialize)]
pub(crate) struct BuildJson<'a> {
    branch: &'a str,
    version: &'a str,
    change_number: &'a str,
    build_id: &'a str,
}

impl<'a> From<&'a Build> for BuildJson<'a> {
    fn from(build: &'a Build) -> Self {
        Self {
            branch: build.branch(),
            version: build.version(),
            change_number: build.change_number(),
            build_id: build.build_id(),
        }
    }
}

/// The JSON shape of an Entry.
#[derive(Serialize)]
pub(crate) struct EntryJson<'a> {
    path: &'a str,
    compression: String,
    encrypted: bool,
    compressed_size: u64,
    size: u64,
}

impl<'a> From<&'a Entry> for EntryJson<'a> {
    fn from(entry: &'a Entry) -> Self {
        Self {
            path: entry.path(),
            compression: entry.compression().to_string(),
            encrypted: entry.is_encrypted(),
            compressed_size: entry.compressed_size(),
            size: entry.uncompressed_size(),
        }
    }
}
