// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use p4kx_extraction::CancelToken;

#[derive(Debug, Default)]
pub(super) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub(super) fn wait(&self, cancel: &CancelToken) {
        while !self.open_within(Duration::from_millis(10)) && !cancel.is_cancelled() {}
    }

    fn open_within(&self, pause: Duration) -> bool {
        let open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let (open, _) = self
            .changed
            .wait_timeout_while(open, pause, |open| !*open)
            .unwrap_or_else(PoisonError::into_inner);
        *open
    }

    fn release(&self) {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.changed.notify_all();
    }
}

/// Keeps the Interior step of the builds of an [`AssemblyView`](super::AssemblyView) from ending
/// until it is dropped, so that a test can look at the building state for as long as it needs.
#[derive(Debug)]
pub struct InteriorHold(pub(super) Arc<Gate>);

impl Drop for InteriorHold {
    fn drop(&mut self) {
        self.0.release();
    }
}
