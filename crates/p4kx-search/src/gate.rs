// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Condvar, Mutex, PoisonError};

/// Bounds the bytes that parallel tasks hold in memory at once.
///
/// A task that needs `size` bytes waits until the tasks holding memory have left room for them; a
/// task asking while none holds any always proceeds, so one request larger than the budget cannot
/// wait forever.
#[derive(Debug)]
pub(crate) struct Gate {
    budget: u64,
    used: Mutex<u64>,
    freed: Condvar,
}

/// Memory taken from a [`Gate`], given back on drop.
#[derive(Debug)]
pub(crate) struct Permit<'g> {
    gate: &'g Gate,
    size: u64,
}

impl Gate {
    pub(crate) const fn new(budget: u64) -> Self {
        Self {
            budget,
            used: Mutex::new(0),
            freed: Condvar::new(),
        }
    }

    pub(crate) fn acquire(&self, size: u64) -> Permit<'_> {
        let used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        let mut used = self
            .freed
            .wait_while(used, |used| {
                *used > 0 && used.saturating_add(size) > self.budget
            })
            .unwrap_or_else(PoisonError::into_inner);
        *used = used.saturating_add(size);
        drop(used);
        Permit { gate: self, size }
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut used = self
            .gate
            .used
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *used = used.saturating_sub(self.size);
        drop(used);
        self.gate.freed.notify_all();
    }
}
