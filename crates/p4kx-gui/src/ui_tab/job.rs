// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::model::describe;

/// Where a background job stands.
pub(super) enum Poll<T> {
    Pending,
    Ready(T),
    Lost,
}

/// A piece of work on a background thread; dropping it abandons the result.
#[derive(Debug)]
pub(super) struct Job<T> {
    result: Receiver<T>,
}

impl<T: Send + 'static> Job<T> {
    /// Runs `work` on a thread named `name` and calls `wake` from it once it ended.
    pub(super) fn spawn(
        name: &str,
        wake: impl Fn() + Send + 'static,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<Self, String> {
        let (sender, result) = mpsc::channel();
        thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                if sender.send(work()).is_ok() {
                    wake();
                }
            })
            .map_err(|error| describe(&error))?;
        Ok(Self { result })
    }

    pub(super) fn poll(&self) -> Poll<T> {
        match self.result.try_recv() {
            Ok(value) => Poll::Ready(value),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => Poll::Lost,
        }
    }
}
