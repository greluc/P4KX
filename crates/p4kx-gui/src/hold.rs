// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

#[derive(Debug, Default)]
pub(crate) struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub(crate) fn wait(&self, cancelled: impl Fn() -> bool) {
        while !self.open_within(Duration::from_millis(10)) && !cancelled() {}
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

/// Keeps a background job from ending until it is dropped, so that a test can look at the running
/// state for as long as it needs.
#[derive(Debug)]
pub struct Hold(Arc<Gate>);

impl Hold {
    pub(crate) fn new() -> (Self, Arc<Gate>) {
        let gate = Arc::new(Gate::default());
        (Self(Arc::clone(&gate)), gate)
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.0.release();
    }
}
