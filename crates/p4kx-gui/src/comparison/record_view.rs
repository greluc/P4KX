// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_comparison::{Filter, OwnedComparison};

use super::record_detail::{Answered, RecordDetail, RecordWorker};
use super::record_filter::{Filtered, RecordFilterJob};
use super::record_list::{RecordChangeColumn, RecordChangeList, RecordChangeRow, RecordChangeSort};

static GENERATIONS: AtomicU64 = AtomicU64::new(0);

/// The list a finished Comparison shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ComparisonList {
    /// The Entries added, removed and changed.
    #[default]
    Entries,
    /// The Records added, removed, moved, changed and unreadable.
    Records,
    /// The structs, properties, enums and enum options of the schema that differ.
    Schema,
}

/// The Record changes of a finished Comparison as the Record list shows them: filtered, sorted
/// and with the field changes of the selected Record.
#[derive(Debug)]
pub struct RecordChangeView {
    list: Arc<RecordChangeList>,
    sort: RecordChangeSort,
    mask: Option<Vec<bool>>,
    pending: Option<RecordFilterJob>,
    shown: Vec<u32>,
    selected: Option<u32>,
    detail: Option<RecordDetail>,
    worker: Option<RecordWorker>,
    reveal: bool,
}

impl RecordChangeView {
    /// Shows every Record of `list` in path order.
    pub(crate) fn new(list: RecordChangeList) -> Self {
        let mut view = Self {
            list: Arc::new(list),
            sort: RecordChangeSort::default(),
            mask: None,
            pending: None,
            shown: Vec::new(),
            selected: None,
            detail: None,
            worker: None,
            reveal: false,
        };
        view.refresh_shown();
        view
    }

    /// Returns every Record change.
    #[must_use]
    pub fn list(&self) -> &RecordChangeList {
        &self.list
    }

    /// Returns the row with id `id`.
    #[must_use]
    pub fn row(&self, id: u32) -> Option<&RecordChangeRow> {
        self.list.row(id)
    }

    /// Returns how the list is sorted.
    #[must_use]
    pub const fn sort(&self) -> RecordChangeSort {
        self.sort
    }

    /// Sorts by `column`, reversing the direction when it is already the sort column.
    pub fn sort_by(&mut self, column: RecordChangeColumn) {
        self.sort = self.sort.clicked(column);
        self.refresh_shown();
    }

    /// Returns the ids of the shown rows in list order.
    #[must_use]
    pub fn shown(&self) -> &[u32] {
        &self.shown
    }

    /// Returns whether a Filter is applied in the background.
    #[must_use]
    pub const fn is_filtering(&self) -> bool {
        self.pending.is_some()
    }

    /// Filters the rows by `filter` on a background thread, calling `wake` from it once the
    /// result is ready; `None` shows every row at once.
    pub(crate) fn set_filter(&mut self, filter: Option<Filter>, wake: impl Fn() + Send + 'static) {
        self.pending = None;
        let Some(filter) = filter else {
            self.show_mask(None);
            return;
        };
        match RecordFilterJob::start(&self.list, &filter, wake) {
            Ok(job) => self.pending = Some(job),
            Err(mask) => self.show_mask(Some(mask)),
        }
    }

    /// Shows no row, as the Filter text a field refuses selects nothing.
    pub(crate) fn show_none(&mut self) {
        self.pending = None;
        self.show_mask(Some(vec![false; self.list.len()]));
    }

    /// Returns the selected row, or `None` when none is.
    #[must_use]
    pub const fn selected(&self) -> Option<u32> {
        self.selected
    }

    /// Selects the row `id`, dropping the field changes of another row.
    pub fn select(&mut self, id: u32) {
        self.reveal |= self.selected != Some(id);
        self.selected = Some(id);
        if self
            .detail
            .as_ref()
            .is_some_and(|detail| detail.row() != id)
        {
            self.detail = None;
        }
    }

    /// Returns the position of the selected row in the list once after each new selection, for
    /// the list to scroll it into view.
    pub fn take_reveal(&mut self) -> Option<usize> {
        let id = self.selected.filter(|_| std::mem::take(&mut self.reveal))?;
        self.shown.iter().position(|&shown| shown == id)
    }

    /// Asks the worker of the view for the fields of the selected Record of `comparison` unless
    /// they are asked for already; the worker starts on the first request and calls `wake` after
    /// each answer.
    pub(crate) fn load_detail(
        &mut self,
        comparison: &Arc<OwnedComparison>,
        wake: impl Fn() + Send + 'static,
    ) {
        let Some(id) = self.selected else {
            self.detail = None;
            return;
        };
        if self
            .detail
            .as_ref()
            .is_some_and(|detail| detail.row() == id)
        {
            return;
        }
        let Some(key) = self.list.row(id).map(RecordChangeRow::key) else {
            return;
        };
        let generation = GENERATIONS.fetch_add(1, Ordering::Relaxed);
        if self.worker.is_none() {
            match RecordWorker::start(comparison, wake) {
                Ok(worker) => self.worker = Some(worker),
                Err(reason) => {
                    self.detail = Some(RecordDetail::failed(id, generation, Some(reason)));
                    return;
                }
            }
        }
        let asked = self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.request(generation, key));
        self.detail = Some(if asked {
            RecordDetail::loading(id, generation)
        } else {
            self.worker = None;
            RecordDetail::failed(id, generation, None)
        });
    }

    /// Returns the field changes of the selected Record, `None` before they were asked for.
    #[must_use]
    pub const fn detail(&self) -> Option<&RecordDetail> {
        self.detail.as_ref()
    }

    /// Takes the results of a finished filter and detail; returns whether one finished.
    pub(crate) fn poll(&mut self) -> bool {
        let detail = self.take_answers();
        let Some(job) = &self.pending else {
            return detail;
        };
        match job.poll() {
            Filtered::Running => detail,
            Filtered::Finished(mask) => {
                self.pending = None;
                self.show_mask(Some(mask));
                true
            }
            Filtered::Lost => {
                self.pending = None;
                true
            }
        }
    }

    fn take_answers(&mut self) -> bool {
        let mut changed = false;
        while let Some(worker) = &self.worker {
            match worker.poll() {
                Answered::Nothing => break,
                Answered::Answer((generation, result)) => {
                    if let Some(detail) = self
                        .detail
                        .as_mut()
                        .filter(|detail| detail.generation() == generation)
                    {
                        detail.finish(result);
                        changed = true;
                    }
                }
                Answered::Gone => {
                    self.worker = None;
                    if let Some(detail) = &mut self.detail {
                        detail.stop();
                    }
                    changed = true;
                }
            }
        }
        changed
    }

    fn show_mask(&mut self, mask: Option<Vec<bool>>) {
        self.mask = mask;
        self.refresh_shown();
    }

    fn refresh_shown(&mut self) {
        let mask = self.mask.as_deref();
        self.shown = self.list.sorted(self.sort, |id| {
            mask.is_none_or(|mask| mask.get(id as usize).copied().unwrap_or(false))
        });
    }
}
