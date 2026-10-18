use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_comparison::{
    CancelToken, DEFAULT_CONTEXT, DiffBody, LineDiffError, NotText, OwnedComparison,
    line_diff_cancellable,
};

use super::diff_document::DiffDocument;
use crate::model::describe;

static GENERATIONS: AtomicU64 = AtomicU64::new(0);

/// Where the line diff of the selected change stands.
#[derive(Debug, Clone, Copy)]
pub enum DiffStatus<'a> {
    /// It is made off the UI thread.
    Loading,
    /// It is ready.
    Ready(&'a DiffDocument),
    /// A side is binary, so there is no line diff.
    Binary,
    /// A side is too large for a line diff.
    TooLarge,
    /// It failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
}

#[derive(Debug)]
enum State {
    Loading(DiffJob),
    Ready(DiffDocument),
    Binary,
    TooLarge,
    Failed(Option<String>),
}

/// The line diff of one change and the hunk the user navigated to.
#[derive(Debug)]
pub struct EntryDiff {
    change: u32,
    generation: u64,
    state: State,
    current: usize,
    scroll: bool,
}

impl EntryDiff {
    /// Starts the line diff of the change `change` at `path` of `comparison` on a background
    /// thread, calling `wake` from it once it ended.
    pub(crate) fn start(
        comparison: &Arc<OwnedComparison>,
        (change, path): (u32, &str),
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        Self {
            change,
            generation: GENERATIONS.fetch_add(1, Ordering::Relaxed),
            state: DiffJob::start(comparison, path, wake)
                .map_or_else(|reason| State::Failed(Some(reason)), State::Loading),
            current: 0,
            scroll: false,
        }
    }

    /// Returns the id of the change it diffs.
    #[must_use]
    pub const fn change(&self) -> u32 {
        self.change
    }

    /// Returns a number no other line diff of this process carries.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns where it stands.
    #[must_use]
    pub fn status(&self) -> DiffStatus<'_> {
        match &self.state {
            State::Loading(_) => DiffStatus::Loading,
            State::Ready(document) => DiffStatus::Ready(document),
            State::Binary => DiffStatus::Binary,
            State::TooLarge => DiffStatus::TooLarge,
            State::Failed(reason) => DiffStatus::Failed(reason.as_deref()),
        }
    }

    /// Returns the hunk navigated to, `None` while there is no hunk.
    #[must_use]
    pub fn current_hunk(&self) -> Option<usize> {
        match &self.state {
            State::Ready(document) if self.current < document.hunks().len() => Some(self.current),
            _ => None,
        }
    }

    /// Moves to the next hunk; stays at the last.
    pub fn next_hunk(&mut self) {
        if let State::Ready(document) = &self.state
            && self.current.saturating_add(1) < document.hunks().len()
        {
            self.current = self.current.saturating_add(1);
            self.scroll = true;
        }
    }

    /// Moves to the previous hunk; stays at the first.
    pub const fn previous_hunk(&mut self) {
        if self.current > 0 && matches!(self.state, State::Ready(_)) {
            self.current = self.current.saturating_sub(1);
            self.scroll = true;
        }
    }

    /// Returns the row of the hunk navigated to once after each move, for the view to scroll to.
    pub fn take_scroll(&mut self) -> Option<u32> {
        let row = match &self.state {
            State::Ready(document) => document.hunks().get(self.current).map(|hunk| hunk.row),
            _ => None,
        };
        std::mem::take(&mut self.scroll).then_some(row).flatten()
    }

    /// Takes the result once the thread ended; returns whether it ended.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Loading(job) = &self.state else {
            return false;
        };
        self.state = match job.result.try_recv() {
            Ok(Made::Ready(document)) => State::Ready(document),
            Ok(Made::Binary) => State::Binary,
            Ok(Made::TooLarge) => State::TooLarge,
            Ok(Made::Failed(reason)) => State::Failed(Some(reason)),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => State::Failed(None),
        };
        true
    }
}

enum Made {
    Ready(DiffDocument),
    Binary,
    TooLarge,
    Failed(String),
}

/// A line diff made on a background thread; dropping it cancels the thread.
struct DiffJob {
    cancel: CancelToken,
    result: Receiver<Made>,
}

impl std::fmt::Debug for DiffJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffJob").finish_non_exhaustive()
    }
}

impl Drop for DiffJob {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl DiffJob {
    fn start(
        comparison: &Arc<OwnedComparison>,
        path: &str,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        let cancel = CancelToken::new();
        let (sender, result) = mpsc::channel();
        let thread_comparison = Arc::clone(comparison);
        let thread_path = path.to_owned();
        let thread_cancel = cancel.clone();
        thread::Builder::new()
            .name("comparison-diff".to_owned())
            .spawn(move || {
                if let Some(made) = make(&thread_comparison, &thread_path, &thread_cancel)
                    && sender.send(made).is_ok()
                {
                    wake();
                }
            })
            .map_err(|error| describe(&error))?;
        Ok(Self { cancel, result })
    }
}

fn make(comparison: &OwnedComparison, path: &str, cancel: &CancelToken) -> Option<Made> {
    let diff = match line_diff_cancellable(
        comparison.old_archive(),
        comparison.new_archive(),
        path,
        DEFAULT_CONTEXT,
        cancel,
    ) {
        Ok(diff) => diff,
        Err(LineDiffError::Cancelled) => return None,
        Err(error) => return Some(Made::Failed(describe(&error))),
    };
    match diff.into_body() {
        DiffBody::Text(text) => {
            DiffDocument::build(text, &|| cancel.is_cancelled()).map(Made::Ready)
        }
        DiffBody::NotText(NotText::Binary) => Some(Made::Binary),
        DiffBody::NotText(NotText::TooLarge) => Some(Made::TooLarge),
    }
}
