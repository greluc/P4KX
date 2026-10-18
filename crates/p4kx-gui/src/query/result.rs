use std::cmp::Ordering;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use p4kx_extraction::query::{ColumnKind, Row};
use p4kx_extraction::{DataForgeEntry, DataType};
use rayon::slice::ParallelSliceMut;

use super::cell::{Cell, CellLook};
use crate::SortDirection;
use crate::sort::compare_ignoring_case;

/// The number of rows one page of the result table shows.
pub const QUERY_PAGE_ROWS: usize = 500;

static GENERATIONS: AtomicU64 = AtomicU64::new(1);

/// One column of a Query result after the record path: its heading and what its cells hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultColumn {
    heading: String,
    list: bool,
    numeric: bool,
}

impl ResultColumn {
    pub(crate) fn new(heading: &str, kind: &ColumnKind) -> Self {
        let numeric = !kind.is_multi_valued()
            && !kind.data_types().is_empty()
            && kind.data_types().iter().all(|data_type| {
                matches!(
                    data_type,
                    DataType::Int8
                        | DataType::Int16
                        | DataType::Int32
                        | DataType::Int64
                        | DataType::UInt8
                        | DataType::UInt16
                        | DataType::UInt32
                        | DataType::UInt64
                        | DataType::Single
                        | DataType::Double
                )
            });
        Self {
            heading: heading.to_owned(),
            list: kind.is_multi_valued(),
            numeric,
        }
    }

    /// Returns the heading, the path in canonical form.
    #[must_use]
    pub fn heading(&self) -> &str {
        &self.heading
    }

    /// Returns whether every cell is a list, for a path through `[*]` or `[TypeName]`.
    #[must_use]
    pub const fn is_list(&self) -> bool {
        self.list
    }

    /// Returns whether every value is a number, so the column is aligned to the right.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        self.numeric
    }
}
/// One row of a Query result: the record path and one cell per selected path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResultRow {
    record_path: Box<str>,
    cells: Box<[Cell]>,
}

impl ResultRow {
    /// Returns the row of `row`, or `None` when the Record's values cannot be read.
    /// Returns the row of `row` with its cells of the `lists` columns as lists, or `None` when
    /// the Record's values cannot be read.
    pub(crate) fn of(row: &Row<'_>, lists: &[bool], buffer: &mut String) -> Option<Self> {
        let cells = row.cells().ok()?;
        Some(Self {
            record_path: row.record_path().into(),
            cells: cells
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let list = lists.get(index).copied().unwrap_or(false);
                    Cell::of(value.as_ref(), list, buffer)
                })
                .collect(),
        })
    }

    fn compare(&self, other: &Self, column: usize, direction: SortDirection) -> Ordering {
        let Some(index) = column.checked_sub(1) else {
            let order = compare_ignoring_case(&self.record_path, &other.record_path);
            return match direction {
                SortDirection::Ascending => order,
                SortDirection::Descending => order.reverse(),
            };
        };
        match (self.cells.get(index), other.cells.get(index)) {
            (Some(left), Some(right)) => left.compare(right, direction),
            (left, right) => left.is_none().cmp(&right.is_none()),
        }
    }
}

/// The finished result of a Query: its columns, its rows in record path order, how they are
/// sorted for display and the page shown.
pub struct QueryResult {
    query: String,
    columns: Vec<ResultColumn>,
    rows: Vec<ResultRow>,
    failed: u64,
    order: Vec<u32>,
    sort: Option<(usize, SortDirection)>,
    page: usize,
    forge: Arc<DataForgeEntry>,
    archive: PathBuf,
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
        (query, columns): (String, Vec<ResultColumn>),
        rows: Vec<ResultRow>,
        failed: u64,
        (forge, archive): (Arc<DataForgeEntry>, PathBuf),
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
            archive,
            generation: GENERATIONS.fetch_add(1, AtomicOrdering::Relaxed),
        }
    }

    /// Returns the Query text that was run.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns the columns of the selected paths, after the record path.
    #[must_use]
    pub fn columns(&self) -> &[ResultColumn] {
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

    /// Returns how the cell at `position` and `column` is shown.
    #[must_use]
    pub fn look(&self, position: usize, column: usize) -> CellLook {
        column
            .checked_sub(1)
            .and_then(|index| self.row(position)?.cells.get(index))
            .map_or(CellLook::Value, Cell::look)
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
                (Some(left), Some(right)) => left.compare(right, column, direction),
                _ => Ordering::Equal,
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

    /// Returns the Archive the Query ran over.
    pub(crate) fn archive(&self) -> &Path {
        &self.archive
    }

    /// Returns the DataForge the Query ran over.
    pub(crate) const fn forge(&self) -> &Arc<DataForgeEntry> {
        &self.forge
    }
}
