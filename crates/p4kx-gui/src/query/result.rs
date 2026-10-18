use std::cmp::Ordering;
use std::fmt::Write as _;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use p4kx_extraction::query::Row;
use p4kx_extraction::{DataForgeEntry, Value};
use rayon::slice::ParallelSliceMut;

use crate::SortDirection;
use crate::sort::compare_ignoring_case;

/// The number of rows one page of the result table shows.
pub const QUERY_PAGE_ROWS: usize = 500;

static GENERATIONS: AtomicU64 = AtomicU64::new(1);

/// One cell of a Query result, as text with what it sorts by.
#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Number(Box<str>, f64),
    Text(Box<str>),
    Error(Box<str>),
    Empty,
}

impl Cell {
    fn of(value: Option<&Value<'_>>, buffer: &mut String) -> Self {
        let Some(value) = value else {
            return Self::Empty;
        };
        buffer.clear();
        let _infallible = write!(buffer, "{value}");
        let text = Box::from(buffer.as_str());
        match number(value) {
            Some(number) => Self::Number(text, number),
            None if matches!(value, Value::Error(_)) => Self::Error(text),
            None => Self::Text(text),
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Number(text, _) | Self::Text(text) | Self::Error(text) => text,
            Self::Empty => "",
        }
    }

    const fn rank(&self) -> u8 {
        match self {
            Self::Number(..) => 0,
            Self::Text(_) => 1,
            Self::Error(_) => 2,
            Self::Empty => 3,
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(_, left), Self::Number(_, right)) => left.total_cmp(right),
            (Self::Text(left), Self::Text(right)) | (Self::Error(left), Self::Error(right)) => {
                compare_ignoring_case(left, right)
            }
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "sorting needs the order of magnitude, not every digit of a 64-bit integer"
)]
fn number(value: &Value<'_>) -> Option<f64> {
    Some(match *value {
        Value::Int8(value) => value.into(),
        Value::Int16(value) => value.into(),
        Value::Int32(value) => value.into(),
        Value::Int64(value) => value as f64,
        Value::UInt8(value) => value.into(),
        Value::UInt16(value) => value.into(),
        Value::UInt32(value) => value.into(),
        Value::UInt64(value) => value as f64,
        Value::Single(value) => value.into(),
        Value::Double(value) => value,
        _ => return None,
    })
}

/// One row of a Query result: the record path and one cell per selected path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResultRow {
    record_path: Box<str>,
    cells: Box<[Cell]>,
}

impl ResultRow {
    /// Returns the row of `row`, or `None` when the Record's values cannot be read.
    pub(crate) fn of(row: &Row<'_>, buffer: &mut String) -> Option<Self> {
        let cells = row.cells().ok()?;
        Some(Self {
            record_path: row.record_path().into(),
            cells: cells
                .iter()
                .map(|value| Cell::of(value.as_ref(), buffer))
                .collect(),
        })
    }

    fn compare(&self, other: &Self, column: usize) -> Ordering {
        let Some(index) = column.checked_sub(1) else {
            return compare_ignoring_case(&self.record_path, &other.record_path);
        };
        match (self.cells.get(index), other.cells.get(index)) {
            (Some(left), Some(right)) => left.compare(right),
            (left, right) => left.is_none().cmp(&right.is_none()),
        }
    }
}

/// The finished result of a Query: its columns, its rows in record path order, how they are
/// sorted for display and the page shown.
pub struct QueryResult {
    query: String,
    columns: Vec<String>,
    rows: Vec<ResultRow>,
    failed: u64,
    order: Vec<u32>,
    sort: Option<(usize, SortDirection)>,
    page: usize,
    forge: Arc<DataForgeEntry>,
    generation: u64,
}

impl std::fmt::Debug for QueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryResult")
            .field("query", &self.query)
            .field("columns", &self.columns)
            .field("rows", &self.rows.len())
            .field("failed", &self.failed)
            .field("sort", &self.sort)
            .field("page", &self.page)
            .finish_non_exhaustive()
    }
}

impl QueryResult {
    pub(crate) fn new(
        query: String,
        columns: Vec<String>,
        rows: Vec<ResultRow>,
        failed: u64,
        forge: Arc<DataForgeEntry>,
    ) -> Self {
        let order = (0..rows.len())
            .filter_map(|index| u32::try_from(index).ok())
            .collect();
        Self {
            query,
            columns,
            rows,
            failed,
            order,
            sort: None,
            page: 0,
            forge,
            generation: GENERATIONS.fetch_add(1, AtomicOrdering::Relaxed),
        }
    }

    /// Returns the Query text that was run.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns the headings of the selected paths, the columns after the record path.
    #[must_use]
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Returns the number of rows, one per selected Record.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns whether no Record was selected.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the number of Records whose values cannot be read; they have no row.
    #[must_use]
    pub const fn failed(&self) -> u64 {
        self.failed
    }

    /// Returns the text of `column` in the `position`th row of the sorted result: column 0 is
    /// the record path, then one per selected path; empty for a missing value.
    #[must_use]
    pub fn cell(&self, position: usize, column: usize) -> Option<&str> {
        let row = self.row(position)?;
        let Some(index) = column.checked_sub(1) else {
            return Some(&row.record_path);
        };
        row.cells.get(index).map(Cell::text)
    }

    /// Returns whether the cell at `position` and `column` holds a value that cannot be read.
    #[must_use]
    pub fn is_error(&self, position: usize, column: usize) -> bool {
        column
            .checked_sub(1)
            .and_then(|index| self.row(position)?.cells.get(index))
            .is_some_and(|cell| matches!(cell, Cell::Error(_)))
    }

    fn row(&self, position: usize) -> Option<&ResultRow> {
        self.rows.get(*self.order.get(position)? as usize)
    }

    /// Returns the column and direction the rows are sorted by, or `None` in record path order.
    #[must_use]
    pub const fn sort(&self) -> Option<(usize, SortDirection)> {
        self.sort
    }

    /// Sorts by `column`, reversing the direction when it is sorted by it already; ties keep
    /// record path order. Shows the first page.
    pub fn sort_by(&mut self, column: usize) {
        if column > self.columns.len() {
            return;
        }
        let direction = match self.sort {
            Some((current, direction)) if current == column => direction.reversed(),
            _ => SortDirection::Ascending,
        };
        let rows = &self.rows;
        let at = |index: u32| rows.get(index as usize);
        self.order.par_sort_by(|&a, &b| {
            let order = match (at(a), at(b)) {
                (Some(left), Some(right)) => left.compare(right, column),
                _ => Ordering::Equal,
            };
            let order = match direction {
                SortDirection::Ascending => order,
                SortDirection::Descending => order.reverse(),
            };
            order.then(a.cmp(&b))
        });
        self.sort = Some((column, direction));
        self.page = 0;
        self.generation = GENERATIONS.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Returns the page shown, counted from 0.
    #[must_use]
    pub const fn page(&self) -> usize {
        self.page
    }

    /// Returns the number of pages, at least 1.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.rows.len().div_ceil(QUERY_PAGE_ROWS).max(1)
    }

    /// Shows `page`, kept within the pages there are.
    pub fn set_page(&mut self, page: usize) {
        let page = page.min(self.page_count() - 1);
        if page != self.page {
            self.page = page;
            self.generation = GENERATIONS.fetch_add(1, AtomicOrdering::Relaxed);
        }
    }

    /// Returns the positions of the rows on the page shown.
    #[must_use]
    pub fn page_rows(&self) -> Range<usize> {
        let start = self
            .page
            .saturating_mul(QUERY_PAGE_ROWS)
            .min(self.rows.len());
        start..start.saturating_add(QUERY_PAGE_ROWS).min(self.rows.len())
    }

    /// Returns a number that changes whenever the shown rows change, for caches.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the DataForge the Query ran over.
    pub(crate) const fn forge(&self) -> &Arc<DataForgeEntry> {
        &self.forge
    }
}
