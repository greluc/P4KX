use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_comparison::{OwnedComparison, RecordKey};

use super::record_fields::RecordFields;
use crate::model::describe;

static GENERATIONS: AtomicU64 = AtomicU64::new(0);

/// Where the field changes of the selected Record stand.
#[derive(Debug, Clone, Copy)]
pub enum RecordDetailStatus<'a> {
    /// They are compared off the UI thread.
    Loading,
    /// They are ready.
    Ready(&'a RecordFields),
    /// Comparing failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
}

#[derive(Debug)]
enum State {
    Loading(Receiver<Result<RecordFields, String>>),
    Ready(RecordFields),
    Failed(Option<String>),
}

/// The field changes of one Record of the list, compared again on a background thread.
#[derive(Debug)]
pub struct RecordDetail {
    row: u32,
    generation: u64,
    state: State,
}

impl RecordDetail {
    /// Compares the Records at `key` of `comparison` again on a background thread for the row
    /// `row`, calling `wake` from it once it ended.
    pub(crate) fn start(
        comparison: &Arc<OwnedComparison>,
        (row, key): (u32, RecordKey),
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (sender, result) = mpsc::channel();
        let thread_comparison = Arc::clone(comparison);
        let spawned = thread::Builder::new()
            .name("comparison-record".to_owned())
            .spawn(move || {
                let fields = thread_comparison
                    .record_change(key, |change| {
                        change.map(RecordFields::of).unwrap_or_default()
                    })
                    .map_err(|error| describe(&error));
                let _gone = sender.send(fields);
                wake();
            });
        Self {
            row,
            generation: GENERATIONS.fetch_add(1, Ordering::Relaxed),
            state: spawned.map_or_else(
                |error| State::Failed(Some(describe(&error))),
                |_handle| State::Loading(result),
            ),
        }
    }

    /// Returns the id of the row it compares.
    #[must_use]
    pub const fn row(&self) -> u32 {
        self.row
    }

    /// Returns a number no other detail of this process carries.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns where it stands.
    #[must_use]
    pub fn status(&self) -> RecordDetailStatus<'_> {
        match &self.state {
            State::Loading(_) => RecordDetailStatus::Loading,
            State::Ready(fields) => RecordDetailStatus::Ready(fields),
            State::Failed(reason) => RecordDetailStatus::Failed(reason.as_deref()),
        }
    }

    /// Takes the result once the thread ended; returns whether it ended.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Loading(result) = &self.state else {
            return false;
        };
        self.state = match result.try_recv() {
            Ok(Ok(fields)) => State::Ready(fields),
            Ok(Err(reason)) => State::Failed(Some(reason)),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => State::Failed(None),
        };
        true
    }
}
