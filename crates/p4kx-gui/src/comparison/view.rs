use std::path::PathBuf;
use std::sync::Arc;

use p4kx_comparison::{Comparison, EntryChange, EntryCounts, Filter, OwnedComparison};

use super::filter_job::{self, FilterJob, Matched, Polled, Started};
use super::orders::{ChangeColumn, ChangeOrders, ChangeSort};
use super::report::{self, ReportJob, ReportStatus};
use crate::SortDirection;

/// The counts of the changes below one top-level directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirectorySummary {
    /// The top-level directory as the first change below it spells it; empty for the root.
    pub directory: String,
    /// The counts below it.
    pub counts: EntryCounts,
}

/// The counts of a Comparison per change kind and per top-level directory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Summary {
    /// The counts per change kind.
    pub counts: EntryCounts,
    /// The counts per top-level directory, ordered by directory ignoring case.
    pub directories: Vec<DirectorySummary>,
}

impl Summary {
    /// Takes the counts of `comparison`.
    pub(crate) fn of(comparison: &Comparison<'_>) -> Self {
        Self {
            counts: comparison.entry_counts(),
            directories: comparison
                .directory_counts()
                .into_iter()
                .map(|directory| DirectorySummary {
                    directory: directory.directory.to_owned(),
                    counts: directory.counts,
                })
                .collect(),
        }
    }
}

/// A finished Comparison as the tab shows it: filtered, sorted and with the report it saves.
#[derive(Debug)]
pub struct ComparisonView {
    comparison: Arc<OwnedComparison>,
    orders: ChangeOrders,
    totals: Summary,
    filter_text: String,
    matched: Option<Matched>,
    pending: Option<FilterJob>,
    sort: ChangeSort,
    shown: Vec<u32>,
    selected: Option<u32>,
    report: Option<ReportJob>,
}

impl ComparisonView {
    /// Prepares `comparison` for display; this sorts every change several times and belongs on
    /// a background thread.
    pub(crate) fn new(comparison: Arc<OwnedComparison>) -> Self {
        let orders = ChangeOrders::build(&comparison);
        let totals = Summary::of(&comparison.comparison());
        let mut view = Self {
            comparison,
            orders,
            totals,
            filter_text: String::new(),
            matched: None,
            pending: None,
            sort: ChangeSort::default(),
            shown: Vec::new(),
            selected: None,
            report: None,
        };
        view.refresh_shown();
        view
    }

    /// Returns the Comparison.
    #[must_use]
    pub fn comparison(&self) -> &OwnedComparison {
        &self.comparison
    }

    /// Returns the change with id `id`, its position in path order.
    #[must_use]
    pub fn change(&self, id: u32) -> Option<EntryChange<'_>> {
        self.comparison.entry_change(id as usize)
    }

    /// Returns the counts of the changes the Filter selects, of every change without one.
    #[must_use]
    pub fn summary(&self) -> &Summary {
        self.matched
            .as_ref()
            .map_or(&self.totals, |matched| &matched.summary)
    }

    /// Returns the text of the Filter.
    #[must_use]
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    /// Returns whether a Filter is applied in the background.
    #[must_use]
    pub const fn is_filtering(&self) -> bool {
        self.pending.is_some()
    }

    /// Filters the list by `text` with the rules of the search on a background thread, calling
    /// `wake` from it once the result is ready; empty text shows every change at once.
    pub fn set_filter(&mut self, text: &str, wake: impl Fn() + Send + 'static) {
        text.clone_into(&mut self.filter_text);
        self.pending = None;
        if text.is_empty() {
            self.matched = None;
            self.refresh_shown();
            return;
        }
        match filter_job::start(&self.comparison, &Filter::new(text), wake) {
            Started::Running(job) => self.pending = Some(job),
            Started::Finished(matched) => self.show_matched(matched),
        }
    }

    /// Returns how the list is sorted.
    #[must_use]
    pub const fn sort(&self) -> ChangeSort {
        self.sort
    }

    /// Sorts by `column`, reversing the direction when it is already the sort column.
    pub fn sort_by(&mut self, column: ChangeColumn) {
        self.sort = self.sort.clicked(column);
        self.refresh_shown();
    }

    /// Returns the ids of the shown changes in list order.
    #[must_use]
    pub fn shown(&self) -> &[u32] {
        &self.shown
    }

    /// Returns the selected change, or `None` when none is.
    #[must_use]
    pub const fn selected(&self) -> Option<u32> {
        self.selected
    }

    /// Selects the change `id`.
    pub const fn select(&mut self, id: u32) {
        self.selected = Some(id);
    }

    /// Writes the JSON report of the shown changes to `path` on a background thread, the same
    /// report `p4kx compare --report` writes with the current Filter; `wake` is called from that
    /// thread once it ended. Nothing starts while a report is being saved.
    pub fn save_report(&mut self, path: PathBuf, wake: impl Fn() + Send + 'static) {
        if self.is_saving() {
            return;
        }
        let filter = (!self.filter_text.is_empty()).then(|| Filter::new(&self.filter_text));
        self.report = Some(report::start(&self.comparison, filter, path, wake));
    }

    /// Returns where saving the report stands, or `None` when none was saved.
    #[must_use]
    pub fn report_status(&self) -> Option<&ReportStatus> {
        self.report.as_ref().map(ReportJob::status)
    }

    /// Returns whether a report is being saved.
    #[must_use]
    pub fn is_saving(&self) -> bool {
        matches!(self.report_status(), Some(ReportStatus::Saving(_)))
    }

    /// Takes the results of a finished filter and report; returns whether one finished.
    pub(crate) fn poll(&mut self) -> bool {
        let saved = self.report.as_mut().is_some_and(ReportJob::poll);
        let Some(job) = &self.pending else {
            return saved;
        };
        match job.poll() {
            Polled::Running => saved,
            Polled::Finished(matched) => {
                self.pending = None;
                self.show_matched(matched);
                true
            }
            Polled::Lost => {
                self.pending = None;
                true
            }
        }
    }

    fn show_matched(&mut self, matched: Matched) {
        self.matched = Some(matched);
        self.refresh_shown();
    }

    fn refresh_shown(&mut self) {
        let ascending = self.orders.ascending(self.sort.column).iter().copied();
        let mask = self.matched.as_ref().map(|matched| matched.mask.as_slice());
        let keep =
            |id: &u32| mask.is_none_or(|mask| mask.get(*id as usize).copied().unwrap_or(false));
        self.shown = match self.sort.direction {
            SortDirection::Ascending => ascending.filter(keep).collect(),
            SortDirection::Descending => ascending.rev().filter(keep).collect(),
        };
    }
}
