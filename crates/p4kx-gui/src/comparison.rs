use std::path::{Path, PathBuf};
use std::sync::Weak;

use p4kx_channels::ChannelKind;
use p4kx_comparison::Levels;
use p4kx_extraction::ArchiveKey;

mod diff;
mod diff_document;
mod filter_job;
mod orders;
mod record_detail;
mod record_fields;
mod record_filter;
mod record_list;
mod record_view;
mod report;
mod run;
mod schema_view;
mod selection;
mod view;

pub use diff::{DiffStatus, EntryDiff};
pub(crate) use diff_document::shown_len;
pub use diff_document::{DiffCell, DiffDocument, DiffHunk, DiffRow};
pub(crate) use orders::rank;
pub use orders::{ChangeColumn, ChangeSort};
pub use record_detail::{RecordDetail, RecordDetailStatus};
pub use record_fields::{FieldName, FieldRow, FieldValue, RecordFields, SideError};
pub(crate) use record_list::kind_rank;
pub use record_list::{RecordChangeColumn, RecordChangeList, RecordChangeRow, RecordChangeSort};
pub use record_view::{ComparisonList, RecordChangeView};
pub use report::ReportStatus;
pub use schema_view::{SchemaRow, SchemaRowKind, SchemaView};
pub use view::{ComparisonView, DirectorySummary, Summary};

use run::{Outcome, Run};
pub(crate) use selection::{ChangeSelection, side_archive};

/// One of the two Archives of a Comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonSide {
    /// The Archive the changes start from.
    Old,
    /// The Archive the changes lead to.
    New,
}

/// The levels the user switched on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComparisonLevels {
    /// Compares the Entries.
    pub entries: bool,
    /// Compares the Records of both DataForges.
    pub records: bool,
}

impl Default for ComparisonLevels {
    fn default() -> Self {
        Self {
            entries: true,
            records: false,
        }
    }
}

impl ComparisonLevels {
    /// Returns the levels as the Comparison service takes them.
    #[must_use]
    pub const fn service(self) -> Levels {
        Levels::NONE
            .with_entries(self.entries)
            .with_records(self.records)
    }
}

/// How far a running Comparison has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonStage {
    /// Both Archives are being opened.
    Opening,
    /// The Entries of both sides are being joined; `done` of `total`.
    Comparing {
        /// Entries joined so far.
        done: usize,
        /// Entries of both sides.
        total: usize,
    },
    /// The Records both sides have are being compared field by field; `done` of `total`.
    ComparingRecords {
        /// Records compared so far.
        done: usize,
        /// Records both sides have.
        total: usize,
    },
    /// The list is being sorted for display.
    Preparing,
}

/// Where the Comparison of the tab stands.
#[derive(Debug, Clone, Copy)]
pub enum ComparisonStatus<'a> {
    /// No Comparison ran yet.
    Idle,
    /// It runs off the UI thread.
    Running(ComparisonStage),
    /// It was cancelled.
    Cancelled,
    /// It failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// It finished.
    Finished(&'a ComparisonView),
}

fn test_channels(kinds: [Option<ChannelKind>; 2]) -> Option<String> {
    let [old, new] = kinds.map(|kind| kind.filter(|kind| kind.is_test_build()));
    match (
        old.and_then(ChannelKind::name),
        new.and_then(ChannelKind::name),
    ) {
        (Some(old), Some(new)) if old != new => Some(format!("{old}, {new}")),
        (Some(name), _) | (None, Some(name)) => Some(name.to_owned()),
        (None, None) => None,
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Running(Run),
    Cancelled,
    Failed(Option<String>),
    Finished(Box<ComparisonView>),
}

/// The Comparison tab: the two picked Archives, the levels and the running or finished
/// Comparison.
#[derive(Debug, Default)]
pub struct ComparisonTab {
    old: Option<PathBuf>,
    new: Option<PathBuf>,
    levels: ComparisonLevels,
    state: State,
    stopping: Option<Weak<()>>,
    key: Option<ArchiveKey>,
    key_wanted: bool,
    kinds: [Option<ChannelKind>; 2],
    test_channels: Option<String>,
}

impl ComparisonTab {
    /// Returns the names of the test Channels the picked Archives live in, such as `PTU` or
    /// `PTU, EPTU`, or `None` when neither lives in one.
    #[must_use]
    pub fn test_channels(&self) -> Option<&str> {
        self.test_channels.as_deref()
    }

    /// Sets the key the picked Archives are opened with.
    pub const fn set_key(&mut self, key: Option<ArchiveKey>) {
        self.key = key;
    }

    /// Returns whether the last Comparison failed for lack of a key that fits, once.
    pub fn take_key_request(&mut self) -> bool {
        std::mem::take(&mut self.key_wanted)
    }

    /// Returns the Archive picked for `side`, or `None` when none is.
    #[must_use]
    pub fn archive(&self, side: ComparisonSide) -> Option<&Path> {
        match side {
            ComparisonSide::Old => self.old.as_deref(),
            ComparisonSide::New => self.new.as_deref(),
        }
    }

    /// Picks the Archive at `path` for `side`.
    pub fn set_archive(&mut self, side: ComparisonSide, path: PathBuf) {
        let kind = Some(ChannelKind::of_archive(&path));
        match side {
            ComparisonSide::Old => {
                self.old = Some(path);
                self.kinds[0] = kind;
            }
            ComparisonSide::New => {
                self.new = Some(path);
                self.kinds[1] = kind;
            }
        }
        self.test_channels = test_channels(self.kinds);
    }

    /// Returns the levels switched on.
    #[must_use]
    pub const fn levels(&self) -> ComparisonLevels {
        self.levels
    }

    /// Switches the levels.
    pub const fn set_levels(&mut self, levels: ComparisonLevels) {
        self.levels = levels;
    }

    /// Returns whether a Comparison runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Returns whether [`Self::start`] would start: both Archives picked, a level on and no
    /// Comparison running.
    #[must_use]
    pub const fn can_start(&self) -> bool {
        self.old.is_some()
            && self.new.is_some()
            && (self.levels.entries || self.levels.records)
            && !self.is_running()
    }

    /// Starts comparing the picked Archives off the UI thread, calling `wake` from it as it
    /// advances and once it ended; returns whether it started.
    pub fn start(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        if !self.can_start() {
            return false;
        }
        let (Some(old), Some(new)) = (&self.old, &self.new) else {
            return false;
        };
        self.state = match run::start(
            (old.clone(), new.clone()),
            self.levels.service(),
            self.key.clone(),
            wake,
        ) {
            Ok(run) => State::Running(run),
            Err(reason) => State::Failed(Some(reason)),
        };
        true
    }

    /// Stops the running Comparison; the tab shows it as cancelled at once.
    pub fn cancel(&mut self) {
        if let State::Running(run) = &self.state {
            self.stopping = Some(run.cancel());
            self.state = State::Cancelled;
        }
    }

    /// Returns whether the thread of a cancelled Comparison still winds down; its Archives are
    /// released once it ended.
    #[must_use]
    pub fn is_stopping(&self) -> bool {
        self.stopping
            .as_ref()
            .is_some_and(|alive| alive.strong_count() > 0)
    }

    /// Takes the outcome of a Comparison that ended and of the view's background work; returns
    /// whether something changed.
    pub fn poll(&mut self) -> bool {
        match &mut self.state {
            State::Running(run) => match run.poll() {
                None => false,
                Some(Outcome::Failed(reason)) => {
                    self.key_wanted |= run.key_asked();
                    self.state = State::Failed(reason);
                    true
                }
                Some(Outcome::Finished(view)) => {
                    self.state = State::Finished(view);
                    true
                }
                Some(Outcome::Cancelled) => {
                    self.state = State::Cancelled;
                    true
                }
            },
            State::Finished(view) => view.poll(),
            State::Idle | State::Cancelled | State::Failed(_) => false,
        }
    }

    /// Returns where the Comparison stands.
    #[must_use]
    pub fn status(&self) -> ComparisonStatus<'_> {
        match &self.state {
            State::Idle => ComparisonStatus::Idle,
            State::Running(run) => ComparisonStatus::Running(run.stage()),
            State::Cancelled => ComparisonStatus::Cancelled,
            State::Failed(reason) => ComparisonStatus::Failed(reason.as_deref()),
            State::Finished(view) => ComparisonStatus::Finished(view),
        }
    }

    /// Returns the finished Comparison, or `None` when there is none.
    #[must_use]
    pub fn view(&self) -> Option<&ComparisonView> {
        match &self.state {
            State::Finished(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the finished Comparison for changes.
    pub fn view_mut(&mut self) -> Option<&mut ComparisonView> {
        match &mut self.state {
            State::Finished(view) => Some(view),
            _ => None,
        }
    }
}
