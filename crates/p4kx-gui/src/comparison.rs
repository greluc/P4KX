use std::path::{Path, PathBuf};

mod filter_job;
mod orders;
mod report;
mod run;
mod view;

pub(crate) use orders::rank;
pub use orders::{ChangeColumn, ChangeSort};
pub use report::ReportStatus;
pub use view::{ComparisonView, DirectorySummary, Summary};

use run::{Outcome, Run};

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
    /// Compares the Records, once the service offers that level.
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
}

impl ComparisonTab {
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
        match side {
            ComparisonSide::Old => self.old = Some(path),
            ComparisonSide::New => self.new = Some(path),
        }
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

    /// Returns whether [`Self::start`] would start: both Archives picked, the Entry level on and
    /// no Comparison running.
    #[must_use]
    pub const fn can_start(&self) -> bool {
        self.old.is_some() && self.new.is_some() && self.levels.entries && !self.is_running()
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
        self.state = match run::start(old.clone(), new.clone(), wake) {
            Ok(run) => State::Running(run),
            Err(reason) => State::Failed(Some(reason)),
        };
        true
    }

    /// Stops the running Comparison; the tab shows it as cancelled at once.
    pub fn cancel(&mut self) {
        if let State::Running(run) = &self.state {
            run.cancel();
            self.state = State::Cancelled;
        }
    }

    /// Takes the outcome of a Comparison that ended and of the view's background work; returns
    /// whether something changed.
    pub fn poll(&mut self) -> bool {
        match &mut self.state {
            State::Running(run) => match run.poll() {
                None => false,
                Some(Outcome::Finished(view)) => {
                    self.state = State::Finished(view);
                    true
                }
                Some(Outcome::Failed(reason)) => {
                    self.state = State::Failed(reason);
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
