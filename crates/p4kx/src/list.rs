use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_extraction::Archive;

use crate::cli::ListArgs;

/// Prints a header and one row per Entry: method, encrypted, compressed size, size, path.
pub(crate) fn run(args: &ListArgs) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let mut out = BufWriter::new(io::stdout().lock());
    write_row(
        &mut out,
        ["METHOD", "ENCRYPTED", "COMPRESSED", "SIZE", "PATH"],
    )?;
    for entry in archive.entries() {
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
    out.flush()?;
    Ok(())
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
