use std::io::{self, Write};

use p4kx_extraction::query::{QueryRun, Row};
use p4kx_extraction::{QUERY_PAGE_SIZE, Value};

/// The number of rows written and of Records whose row failed.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Counts {
    pub(crate) records: u64,
    pub(crate) failed: u64,
}

impl Counts {
    /// Returns `<n> Records<after>`, followed by `, <m> failed` when a Record failed.
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
        format!("{} {noun}{after}{failed}", self.records)
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
    for page in run.pages(QUERY_PAGE_SIZE) {
        for row in &page {
            if let Ok(cells) = row.cells() {
                write(row, cells)?;
                counts.records = counts.records.saturating_add(1);
            } else {
                counts.failed = counts.failed.saturating_add(1);
                report_failed(row);
            }
        }
    }
    Ok(())
}

/// Reports on stderr that the values of `row`'s Record cannot be read; a report that cannot be
/// written is dropped.
pub(crate) fn report_failed(row: &Row<'_>) {
    if let Err(error) = row.cells() {
        let _unreported = writeln!(
            io::stderr().lock(),
            "failed {} ({}): {error}",
            row.record_path(),
            row.record().guid()
        );
    }
}
