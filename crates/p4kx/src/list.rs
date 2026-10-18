use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_extraction::{Archive, Entry};

use crate::cli::ListArgs;

/// Prints a header and one row per Entry: method, encrypted, compressed size, size, path.
pub(crate) fn run(args: &ListArgs) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "{:<10} {:<9} {:>14} {:>14}  PATH",
        "METHOD", "ENCRYPTED", "COMPRESSED", "SIZE"
    )?;
    for entry in archive.entries() {
        write_row(&mut out, entry)?;
    }
    out.flush()?;
    Ok(())
}

fn write_row(out: &mut impl Write, entry: &Entry) -> io::Result<()> {
    let encrypted = if entry.is_encrypted() { "yes" } else { "no" };
    writeln!(
        out,
        "{:<10} {:<9} {:>14} {:>14}  {}",
        entry.compression().to_string(),
        encrypted,
        entry.compressed_size(),
        entry.uncompressed_size(),
        entry.path()
    )
}
