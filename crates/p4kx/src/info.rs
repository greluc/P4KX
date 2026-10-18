use std::io::{self, Write};

use anyhow::Result;
use p4kx_channels::channel_name;
use p4kx_extraction::{Archive, Build, Entry};

use crate::cli::InfoArgs;

const UNKNOWN: &str = "unknown";

/// Prints the Channel, the Build and a summary of the Archive, one `Label: value` line each.
pub(crate) fn run(args: &InfoArgs) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let channel = channel_name(archive.path());
    let mut lines: Vec<(&str, String)> = vec![
        ("Archive", archive.path().display().to_string()),
        ("Channel", channel.unwrap_or_else(|| UNKNOWN.to_owned())),
    ];
    if let Some(build) = archive.build() {
        lines.extend([
            ("Branch", build.branch().to_owned()),
            ("Version", build.version().to_owned()),
            ("Change number", build.change_number().to_owned()),
            ("BuildId", build.build_id().to_owned()),
        ]);
    } else {
        warn_about_manifest(&archive)?;
        lines.push(("Build", UNKNOWN.to_owned()));
    }
    let entries = archive.entries();
    let encrypted = entries.iter().filter(|entry| entry.is_encrypted()).count();
    let compressed: u64 = entries.iter().map(Entry::compressed_size).sum();
    let size: u64 = entries.iter().map(Entry::uncompressed_size).sum();
    lines.extend([
        ("Entries", entries.len().to_string()),
        ("Encrypted", encrypted.to_string()),
        ("Compressed size", compressed.to_string()),
        ("Size", size.to_string()),
    ]);
    let mut out = io::stdout().lock();
    for (label, value) in lines {
        writeln!(out, "{:<16} {value}", format!("{label}:"))?;
    }
    Ok(())
}

fn warn_about_manifest(archive: &Archive) -> io::Result<()> {
    let Some(folder) = archive.path().parent() else {
        return Ok(());
    };
    if let Err(error) = Build::read_from(folder) {
        let cause = std::error::Error::source(&error).map(ToString::to_string);
        let mut err = io::stderr().lock();
        match cause {
            Some(cause) => writeln!(err, "warning: {error}: {cause}")?,
            None => writeln!(err, "warning: {error}")?,
        }
    }
    Ok(())
}
