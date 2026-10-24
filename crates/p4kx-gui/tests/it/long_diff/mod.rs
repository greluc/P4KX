// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A pair of Archives whose only Entry is a long text of which every line changed.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use tempfile::TempDir;

use crate::diffing::Pair;

/// The changed Entry.
pub(crate) const LONG: &str = "Data\\long.txt";

/// The number of lines of the Entry.
pub(crate) const LINES: u32 = 300;

fn text(prefix: &str) -> String {
    (1..=LINES).fold(String::new(), |mut text, line| {
        text.push_str(prefix);
        text.push_str("line ");
        text.push_str(&line.to_string());
        text.push('\n');
        text
    })
}

fn write(
    dir: &Path,
    channel: &str,
    version: &str,
    text: String,
) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    fs::create_dir_all(&folder)?;
    write_build_manifest(&folder, "sc-alpha-4.3", version, "10203", "2026")?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(LONG, text))
        .write_to(&path)?;
    Ok(path)
}

/// Writes LIVE and PTU into a new temporary folder.
pub(crate) fn pair() -> Result<(TempDir, Pair), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let old = write(dir.path(), "LIVE", "4.3.1.10203", text(""))?;
    let new = write(dir.path(), "PTU", "4.3.2.10400", text("changed "))?;
    Ok((dir, Pair { old, new }))
}
