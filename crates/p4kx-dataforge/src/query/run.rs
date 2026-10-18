use std::num::NonZeroUsize;
use std::slice::Chunks;

use rayon::prelude::*;

use crate::build::instance;
use crate::dataforge::out_of_range;
use crate::query::CompiledQuery;
use crate::query::compile::Plan;
use crate::{DataForgeError, Record, Value};

/// The Records a compiled Query selects, ordered by record path and then by definition order;
/// their rows are built page by page.
#[derive(Debug)]
pub struct QueryRun<'q, 'f> {
    query: &'q CompiledQuery<'f>,
    records: Vec<u32>,
}

impl<'f> CompiledQuery<'f> {
    /// Selects the Records by type and record path and keeps those that meet the condition, in
    /// parallel over every Record.
    ///
    /// Records whose record path cannot be read are never selected. A Record whose values cannot
    /// be read to decide the condition is kept, and its row reports why.
    #[must_use]
    pub fn run(&self) -> QueryRun<'_, 'f> {
        let records = self
            .forge
            .path_order()
            .par_iter()
            .copied()
            .filter(|&record| self.selects(record))
            .collect();
        QueryRun {
            query: self,
            records,
        }
    }

    fn selects(&self, record: u32) -> bool {
        self.plan_of(record).is_some()
            && self.pattern.as_ref().is_none_or(|pattern| {
                self.forge
                    .path_of(record)
                    .is_some_and(|path| pattern.matches(path))
            })
            && self.meets_condition(record)
    }

    /// Returns whether the Record meets the condition or cannot be read to decide it.
    fn meets_condition(&self, record: u32) -> bool {
        let Some(predicate) = &self.predicate else {
            return true;
        };
        match self.plan_of(record) {
            Some(Plan::Read { tests, .. }) => self.instance_of(record).map_or(true, |bytes| {
                predicate.eval(self.forge, tests, bytes) == Some(true)
            }),
            Some(Plan::Broken(_)) => true,
            None => false,
        }
    }

    fn plan_of(&self, record: u32) -> Option<&Plan> {
        let def = self.forge.record_def(record)?;
        self.plans
            .get(usize::try_from(def.struct_index).ok()?)?
            .as_ref()
    }
}

impl<'f> QueryRun<'_, 'f> {
    /// Returns the number of selected Records, one row each.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether no Record is selected.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the record paths of the selected Records in row order.
    pub fn record_paths(&self) -> impl Iterator<Item = &'f str> + '_ {
        let forge = self.query.forge;
        self.records
            .iter()
            .filter_map(move |&record| forge.path_of(record))
    }

    /// Returns the rows in pages of at most `page_size`; each page is built in parallel when it
    /// is reached.
    #[must_use]
    pub fn pages(&self, page_size: NonZeroUsize) -> Pages<'_, 'f> {
        Pages {
            query: self.query,
            chunks: self.records.chunks(page_size.get()),
        }
    }
}

/// The pages of a [`QueryRun`], in order.
#[derive(Debug)]
pub struct Pages<'r, 'f> {
    query: &'r CompiledQuery<'f>,
    chunks: Chunks<'r, u32>,
}

impl<'f> Iterator for Pages<'_, 'f> {
    type Item = Vec<Row<'f>>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunks.next()?;
        Some(
            chunk
                .par_iter()
                .filter_map(|&record| self.query.row(record))
                .collect(),
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.size_hint()
    }
}

/// One row of a Query result: a selected Record and its values, one per column.
#[derive(Debug, Clone)]
pub struct Row<'f> {
    record: Record<'f>,
    record_path: &'f str,
    cells: Result<Vec<Option<Value<'f>>>, DataForgeError>,
}

impl<'f> Row<'f> {
    /// Returns the selected Record.
    #[must_use]
    pub const fn record(&self) -> Record<'f> {
        self.record
    }

    /// Returns the record path, the first column.
    #[must_use]
    pub const fn record_path(&self) -> &'f str {
        self.record_path
    }

    /// Returns the values of the columns after the record path; `None` where the Record's type
    /// has no such field.
    ///
    /// # Errors
    ///
    /// Returns the [`DataForgeError`] [`Record::tree`] would return when the Record's layout or
    /// instance cannot be read.
    pub fn cells(&self) -> Result<&[Option<Value<'f>>], &DataForgeError> {
        self.cells.as_deref()
    }
}

impl<'f> CompiledQuery<'f> {
    fn row(&self, index: u32) -> Option<Row<'f>> {
        let record = self.forge.record(index)?;
        let record_path = self.forge.path_of(index)?;
        let cells = match self.plan_of(index)? {
            _ if self.columns.is_empty() && self.predicate.is_none() => Ok(Vec::new()),
            Plan::Broken(error) => Err(error.clone()),
            Plan::Read { columns, .. } => self.instance_of(index).map(|bytes| {
                columns
                    .iter()
                    .map(|column| column.as_ref()?.read(self.forge, bytes))
                    .collect()
            }),
        };
        Some(Row {
            record,
            record_path,
            cells,
        })
    }

    /// Returns the bytes of the Record's instance, which the Record tree reads its values from.
    fn instance_of(&self, index: u32) -> Result<&'f [u8], DataForgeError> {
        let forge = self.forge;
        let def = forge
            .record_def(index)
            .ok_or_else(|| out_of_range("Record", index as usize, "Record", index.into()))?;
        let structure = usize::try_from(def.struct_index)
            .ok()
            .and_then(|struct_index| forge.structs.get(struct_index))
            .ok_or_else(|| {
                out_of_range("Record", index as usize, "struct", def.struct_index.into())
            })?;
        instance(forge, index, def, structure)
    }
}
