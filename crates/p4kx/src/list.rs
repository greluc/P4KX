use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_extraction::{Archive, Entry, Filter};
use serde::Serialize;

use crate::cli::{ListArgs, SearchArgs};
use crate::json::{self, EntryJson};

#[derive(Serialize)]
struct EntriesJson<'a> {
    archive: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<&'a str>,
    entries: Vec<EntryJson<'a>>,
}

/// Prints every Entry of the Archive.
pub(crate) fn run(args: &ListArgs, as_json: bool) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let entries: Vec<&Entry> = archive.entries().iter().collect();
    print(archive.path(), None, &entries, as_json)
}

/// Prints the Entries of the Archive that match the Filter.
pub(crate) fn search(args: &SearchArgs, as_json: bool) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let filter = Filter::new(&args.filter);
    print(
        archive.path(),
        Some(&filter),
        &filter.select(&archive),
        as_json,
    )
}

fn print(archive: &Path, filter: Option<&Filter>, entries: &[&Entry], as_json: bool) -> Result<()> {
    if as_json {
        json::print(&EntriesJson {
            archive: json::path(archive),
            filter: filter.map(Filter::as_str),
            entries: entries.iter().copied().map(EntryJson::from).collect(),
        })?;
    } else {
        print_table(entries)?;
    }
    Ok(())
}

/// Prints a header and one row per Entry: method, encrypted, compressed size, size, path.
pub(crate) fn print_table(entries: &[&Entry]) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    write_row(
        &mut out,
        ["METHOD", "ENCRYPTED", "COMPRESSED", "SIZE", "PATH"],
    )?;
    for entry in entries {
        let encrypted = if entry.is_encrypted() { "yes" } else { "no" };
        write_row(
            &mut out,
            [
                &entry.compression().to_string(),
                encrypted,
                &entry.compressed_size().to_string(),
                &entry.uncompressed_size().to_string(),
                entry.path(),
            ],
        )?;
    }
    out.flush()
}

fn write_row(
    out: &mut impl Write,
    [method, encrypted, compressed, size, path]: [&str; 5],
) -> io::Result<()> {
    writeln!(
        out,
        "{method:<10} {encrypted:<9} {compressed:>14} {size:>14}  {path}"
    )
}
