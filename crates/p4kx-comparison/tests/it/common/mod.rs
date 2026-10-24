// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the line diff tests: a pair of synthetic Archives and the rendered hunks.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_comparison::{Archive, DiffBody, LineDiff, LineKind, TextDiff};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

fn write(dir: &Path, channel: &str, builder: &ArchiveBuilder) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    std::fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(path)
}

pub(crate) fn pair(
    old: Vec<TestEntry>,
    new: Vec<TestEntry>,
) -> Result<(tempfile::TempDir, Archive, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let old = old
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry);
    let new = new
        .into_iter()
        .fold(ArchiveBuilder::v2(), ArchiveBuilder::entry);
    let old = Archive::open_with_key(
        write(dir.path(), "LIVE", &old)?,
        Some(p4kx_testkit::test_key()),
    )?;
    let new = Archive::open_with_key(
        write(dir.path(), "PTU", &new)?,
        Some(p4kx_testkit::test_key()),
    )?;
    Ok((dir, old, new))
}

pub(crate) fn rendered(diff: &LineDiff<'_>) -> Vec<Vec<(LineKind, String)>> {
    let Ok(text) = text_diff(diff) else {
        return Vec::new();
    };
    text.hunks()
        .map(|hunk| {
            hunk.lines()
                .map(|line| (line.kind(), line.text().into_owned()))
                .collect()
        })
        .collect()
}

pub(crate) fn text_diff<'d>(diff: &'d LineDiff<'_>) -> Result<&'d TextDiff, Box<dyn Error>> {
    match diff.body() {
        DiffBody::Text(text) => Ok(text),
        DiffBody::NotText(reason) => Err(format!("text expected, got {reason:?}").into()),
    }
}
