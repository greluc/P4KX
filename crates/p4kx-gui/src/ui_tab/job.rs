// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::CancelToken;

use crate::model::describe;

/// Where a background job stands.
pub(super) enum Poll<T> {
    Pending,
    Ready(T),
    Lost,
}

/// A piece of work on a background thread; dropping it abandons the result and asks the work to
/// stop.
#[derive(Debug)]
pub(super) struct Job<T> {
    result: Receiver<T>,
    cancel: CancelToken,
}

impl<T: Send + 'static> Job<T> {
    /// Runs `work` on a thread named `name` and calls `wake` from it once it ended.
    pub(super) fn spawn(
        name: &str,
        wake: impl Fn() + Send + 'static,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<Self, String> {
        Self::spawn_stoppable(name, wake, move |_| work())
    }

    /// Runs `work` like [`Job::spawn`], handing it the token that [`Job::stop`] and dropping the
    /// job cancel.
    pub(super) fn spawn_stoppable(
        name: &str,
        wake: impl Fn() + Send + 'static,
        work: impl FnOnce(&CancelToken) -> T + Send + 'static,
    ) -> Result<Self, String> {
        let (sender, result) = mpsc::channel();
        let cancel = CancelToken::new();
        let token = cancel.clone();
        thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                if sender.send(work(&token)).is_ok() {
                    wake();
                }
            })
            .map_err(|error| describe(&error))?;
        Ok(Self { result, cancel })
    }

    /// Asks the work to stop; the result still arrives.
    pub(super) fn stop(&self) {
        self.cancel.cancel();
    }

    pub(super) fn poll(&self) -> Poll<T> {
        match self.result.try_recv() {
            Ok(value) => Poll::Ready(value),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => Poll::Lost,
        }
    }
}

impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
