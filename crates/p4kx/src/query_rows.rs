use std::io::{self, Write};
use std::num::NonZeroUsize;

use p4kx_extraction::Value;
use p4kx_extraction::query::{QueryRun, Row};

/// The number of rows built and written at a time.
pub(crate) const PAGE_SIZE: NonZeroUsize = match NonZeroUsize::new(1024) {
    Some(size) => size,
    None => NonZeroUsize::MIN,
};

/// The number of rows written, of Records whose row failed and of the unresolved References in
/// the rows written.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Counts {
    pub(crate) records: u64,
    pub(crate) failed: u64,
    pub(crate) unresolved: u64,
}

impl Counts {
    /// Returns `<n> Records<after>`, followed by `, <m> failed` when a Record failed and by
    /// `, <k> unresolved References` when `->` met a null or dangling Reference.
    pub(crate) fn line(&self, after: &str) -> String {
        let noun = if self.records == 1 {
            "Record"
        } else {
            "Records"
        };
        let failed = if self.failed == 0 {
            String::new()
        } else {
            format!(", {} failed", self.failed)
        };
        let unresolved = match self.unresolved {
            0 => String::new(),
            1 => ", 1 unresolved Reference".to_owned(),
            count => format!(", {count} unresolved References"),
        };
        format!("{} {noun}{after}{failed}{unresolved}", self.records)
    }
}

/// Calls `write` with each row's record path and values, page by page, and reports on stderr
/// each Record whose values cannot be read; `counts` holds what was done also after an error.
///
/// # Errors
///
/// Returns the [`io::Error`] of `write`; a report that cannot be written to stderr is dropped.
pub(crate) fn for_each_row(
    run: &QueryRun<'_, '_>,
    counts: &mut Counts,
    mut write: impl FnMut(&Row<'_>, &[Option<Value<'_>>]) -> io::Result<()>,
) -> io::Result<()> {
    for page in run.pages(PAGE_SIZE) {
        for row in &page {
            match row.cells() {
                Ok(cells) => {
                    write(row, cells)?;
                    counts.records = counts.records.saturating_add(1);
                    counts.unresolved = counts.unresolved.saturating_add(
                        u64::try_from(row.unresolved_references()).unwrap_or(u64::MAX),
                    );
                }
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    let _unreported = writeln!(
                        io::stderr().lock(),
                        "failed {} ({}): {error}",
                        row.record_path(),
                        row.record().guid()
                    );
                }
            }
        }
    }
    Ok(())
}
