// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::CancelToken;

use crate::model::describe;

/// What a task has produced so far.
pub(super) enum Polled<T> {
    Pending,
    Ready(T),
    Lost,
}

/// Work on a background thread whose result is polled; dropping it asks the work to stop.
pub(super) struct Task<T> {
    result: Receiver<T>,
    cancel: CancelToken,
}

impl<T: Send + 'static> Task<T> {
    /// Runs `work` on a thread called `name` and calls `wake` from it once it ended.
    pub(super) fn spawn(
        name: &str,
        wake: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&CancelToken) -> T + Send + 'static,
    ) -> Result<Self, String> {
        let (sender, result) = mpsc::channel();
        let cancel = CancelToken::new();
        let thread_cancel = cancel.clone();
        thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let done = work(&thread_cancel);
                if sender.send(done).is_ok() {
                    wake();
                }
            })
            .map_err(|error| describe(&error))?;
        Ok(Self { result, cancel })
    }
}

impl<T> Task<T> {
    pub(super) fn poll(&self) -> Polled<T> {
        match self.result.try_recv() {
            Ok(done) => Polled::Ready(done),
            Err(TryRecvError::Empty) => Polled::Pending,
            Err(TryRecvError::Disconnected) => Polled::Lost,
        }
    }

    pub(super) fn cancel(&self) {
        self.cancel.cancel();
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub(super) fn wait(&self) -> Option<T> {
        self.result.recv().ok()
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
