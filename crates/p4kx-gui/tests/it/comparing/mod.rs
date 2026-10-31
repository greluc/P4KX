// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the Comparison tests: a pair of synthetic Archives that differ in known ways.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// Two Archives in the Channels LIVE and PTU and the folder holding both.
pub(crate) struct Pair {
    pub(crate) dir: TempDir,
    pub(crate) old: PathBuf,
    pub(crate) new: PathBuf,
}

pub(crate) fn write(
    dir: &Path,
    channel: &str,
    version: &str,
    builder: &ArchiveBuilder,
) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    fs::create_dir_all(&folder)?;
    write_build_manifest(&folder, "sc-alpha-4.3", version, "10203", "2026")?;
    let path = folder.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(path)
}

/// Writes LIVE with five Entries and PTU with one added, one removed and two changed Entries.
pub(crate) fn pair() -> Result<Pair, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let old = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\same.txt", vec![1; 10]))
        .entry(TestEntry::stored("Data\\removed.txt", vec![2; 20]))
        .entry(TestEntry::stored("Data\\Libs\\size.xml", vec![3; 30]))
        .entry(TestEntry::stored("Data\\Libs\\crc.xml", vec![4; 40]))
        .entry(TestEntry::stored("Engine\\keep.cfg", vec![5; 50]));
    let new = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\same.txt", vec![1; 10]))
        .entry(TestEntry::stored("Data\\Libs\\size.xml", vec![3; 3000]))
        .entry(TestEntry::stored("Data\\Libs\\crc.xml", vec![9; 40]))
        .entry(TestEntry::stored("Engine\\keep.cfg", vec![5; 50]))
        .entry(TestEntry::stored("Engine\\added.cfg", vec![6; 60]));
    let old = write(dir.path(), "LIVE", "4.3.1.10203", &old)?;
    let new = write(dir.path(), "PTU", "4.3.2.10400", &new)?;
    Ok(Pair { dir, old, new })
}
