use std::io::{self, Write};

use p4kx_extraction::Archive;

/// How many Entry issues are printed before the rest is only counted.
const SHOWN: usize = 10;

/// Prints the issues of the Archive's Entries to stderr, one `warning:` line each up to a limit
/// and a line for the rest.
pub(crate) fn warn(archive: &Archive) -> io::Result<()> {
    let issues = archive.issues();
    if archive.issue_count() == 0 {
        return Ok(());
    }
    let mut err = io::stderr().lock();
    for (index, issue) in issues.iter().take(SHOWN) {
        let path = archive
            .entries()
            .get(*index)
            .map_or("", |entry| entry.path());
        writeln!(err, "warning: Entry {path}: {issue}")?;
    }
    let shown = issues.len().min(SHOWN);
    if archive.issue_count() > shown {
        writeln!(
            err,
            "warning: {} more Entry issues not shown",
            archive.issue_count() - shown
        )?;
    }
    Ok(())
}
