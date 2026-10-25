// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::Archive;

/// How many Entry issues are printed before the rest is only counted.
const SHOWN: usize = 10;

/// Prints the issues of the Archive's Entries to stderr, one `warning:` line each up to a limit
/// and a line for the rest.
pub(crate) fn warn(archive: &Archive) -> io::Result<()> {
    let issues: Vec<_> = archive
        .issues()
        .iter()
        .filter(|(_, issue)| !issue.is_archive_wide())
        .collect();
    let wide = archive.issues().len() - issues.len();
    let count = archive.issue_count() - wide;
    if count == 0 {
        return Ok(());
    }
    let mut err = io::stderr().lock();
    for (index, issue) in issues.iter().copied().take(SHOWN) {
        let path = archive
            .entries()
            .get(*index)
            .map_or("", |entry| entry.path());
        writeln!(err, "warning: Entry {path}: {issue}")?;
    }
    let shown = issues.len().min(SHOWN);
    if count > shown {
        writeln!(err, "warning: {} more Entry issues not shown", count - shown)?;
    }
    Ok(())
}

/// Prints the issues that concern the whole Archive, such as its recovery, to stderr.
pub(crate) fn warn_archive_wide(archive: &Archive) {
    let mut err = io::stderr().lock();
    for (_, issue) in archive
        .issues()
        .iter()
        .filter(|(_, issue)| issue.is_archive_wide())
    {
        let _ignored = writeln!(err, "warning: {}: {issue}", archive.path().display());
    }
}
