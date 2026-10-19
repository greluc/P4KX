use p4kx_extraction::DataForge;
use p4kx_extraction::query::{CompiledQuery, Query, QueryError, ResultFormat, ResultWriter, Row};

use crate::budget::{Counter, MAX_RESPONSE_LEN, json_len};
use crate::error::{ToolError, chain};
use crate::forge_shapes::{ColumnShape, FailedRow, QueryResult};
use crate::page::Page;

/// The bytes the end of the rows adds: the closing bracket and a line ending.
const CLOSING_LEN: usize = 2;

/// Runs `text` over `forge` and returns one page of its rows: at most the page size, and no
/// more than fit the response cap.
pub(crate) fn run(
    forge: &DataForge<'_>,
    archive: String,
    text: &str,
    page: Page,
) -> Result<QueryResult, ToolError> {
    let query = Query::parse(text).map_err(|error| invalid(&error, text))?;
    let compiled = query
        .compile(forge)
        .map_err(|error| invalid(&error, text))?;
    let selected = compiled.run();
    let total = selected.len();
    let rows = selected.rows(page.offset()..page.offset().saturating_add(page.limit()));
    let written = write_rows(&compiled, &rows)?;
    let next_cursor = page.next_cursor_after(written.taken, total)?;
    Ok(QueryResult {
        archive,
        query: query.to_string(),
        columns: columns(&compiled),
        total,
        rows: serde_json::from_slice(&written.json)?,
        failed: written.failed,
        unresolved_references: written.unresolved,
        next_cursor,
    })
}

fn invalid(error: &QueryError, text: &str) -> ToolError {
    let span = error.span();
    ToolError::Query {
        start: span.start(),
        end: span.end(),
        marked: error.marked(text).to_string(),
    }
}

fn columns(compiled: &CompiledQuery<'_>) -> Vec<ColumnShape> {
    compiled
        .columns()
        .iter()
        .zip(compiled.column_kinds())
        .map(|(name, kind)| ColumnShape {
            name: name.clone(),
            list: kind.is_multi_valued(),
            types: kind.data_types().iter().map(ToString::to_string).collect(),
        })
        .collect()
}

/// The rows of one page as the JSON `p4kx dcb query` writes, and what else the page holds.
struct Written {
    json: Vec<u8>,
    taken: usize,
    failed: Vec<FailedRow>,
    unresolved: usize,
}

/// Writes `rows` as JSON while they and the failed Records fit the response cap; a counting
/// writer runs one row ahead of the output, so a row is only written once it is known to fit.
fn write_rows(compiled: &CompiledQuery<'_>, rows: &[Row<'_>]) -> Result<Written, ToolError> {
    let columns = compiled.columns();
    let mut ahead = ResultWriter::new(Counter::default(), ResultFormat::Json, columns)?;
    let mut out = ResultWriter::new(Vec::new(), ResultFormat::Json, columns)?;
    let mut taken = 0;
    let mut failed = Vec::new();
    let mut failed_len = 0_usize;
    let mut unresolved = 0_usize;
    for row in rows {
        match row.cells() {
            Ok(cells) => {
                ahead.write_row(row.record_path(), cells)?;
                let used = ahead.get_ref().len() + CLOSING_LEN + failed_len;
                if used > MAX_RESPONSE_LEN {
                    if taken == 0 {
                        return Err(ToolError::TooLargeItem {
                            what: format!("row of {}", row.record_path()),
                            limit: MAX_RESPONSE_LEN,
                            hint: "select fewer or narrower paths",
                        });
                    }
                    break;
                }
                out.write_row(row.record_path(), cells)?;
                unresolved = unresolved.saturating_add(row.unresolved_references());
            }
            Err(error) => {
                let row = FailedRow {
                    record_path: row.record_path().to_owned(),
                    name: row.record().name().unwrap_or_default().to_owned(),
                    error: chain(error),
                };
                let len = json_len(&row);
                if out.get_ref().len() + CLOSING_LEN + failed_len + len > MAX_RESPONSE_LEN {
                    break;
                }
                failed_len += len;
                failed.push(row);
            }
        }
        taken += 1;
    }
    Ok(Written {
        json: out.finish()?,
        taken,
        failed,
        unresolved,
    })
}
