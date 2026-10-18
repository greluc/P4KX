use std::sync::Arc;

use p4kx_comparison::{Filter, OwnedComparison};

use super::record_detail::RecordDetail;
use super::record_filter::{Filtered, RecordFilterJob};
use super::record_list::{RecordChangeColumn, RecordChangeList, RecordChangeRow, RecordChangeSort};

/// The list a finished Comparison shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ComparisonList {
    /// The Entries added, removed and changed.
    #[default]
    Entries,
    /// The Records added, removed, moved, changed and unreadable.
    Records,
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

    /// Starts comparing the fields of the selected Record of `comparison` on a background thread
    /// unless that is done already, calling `wake` from it once it ended.
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
        self.detail = self
            .list
            .row(id)
            .map(|row| RecordDetail::start(comparison, (id, row.key()), wake));
    }

    /// Returns the field changes of the selected Record, `None` before they were asked for.
    #[must_use]
    pub const fn detail(&self) -> Option<&RecordDetail> {
        self.detail.as_ref()
    }

    /// Takes the results of a finished filter and detail; returns whether one finished.
    pub(crate) fn poll(&mut self) -> bool {
        let detail = self.detail.as_mut().is_some_and(RecordDetail::poll);
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
