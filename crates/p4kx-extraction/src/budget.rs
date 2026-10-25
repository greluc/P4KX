// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

const STOP_POLL: Duration = Duration::from_millis(50);

/// Units, pixels or bytes, that parallel jobs may hold at the same time; a job waits until enough
/// are free.
#[derive(Debug)]
pub(crate) struct Budget {
    free: Mutex<u64>,
    freed: Condvar,
    total: u64,
}

/// Units taken from a [`Budget`], given back when dropped.
#[derive(Debug)]
pub(crate) struct Lease<'a> {
    budget: &'a Budget,
    units: u64,
}

impl Budget {
    /// Creates a budget of `total` units.
    pub(crate) const fn new(total: u64) -> Self {
        Self {
            free: Mutex::new(total),
            freed: Condvar::new(),
            total,
        }
    }

    /// Waits until `units`, at most the whole budget, are free and takes them, or returns `None`
    /// once `stop` says so; `stop` is asked at least every 50 ms.
    pub(crate) fn take_unless(&self, units: u64, stop: &dyn Fn() -> bool) -> Option<Lease<'_>> {
        let units = units.min(self.total);
        let mut free = self.free.lock().unwrap_or_else(PoisonError::into_inner);
        while *free < units {
            if stop() {
                return None;
            }
            free = self
                .freed
                .wait_timeout(free, STOP_POLL)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        *free -= units;
        drop(free);
        Some(Lease {
            budget: self,
            units,
        })
    }

    /// Waits until `units`, at most the whole budget, are free and takes them.
    pub(crate) fn take(&self, units: u64) -> Lease<'_> {
        let units = units.min(self.total);
        {
            let free = self.free.lock().unwrap_or_else(PoisonError::into_inner);
            let mut free = self
                .freed
                .wait_while(free, |free| *free < units)
                .unwrap_or_else(PoisonError::into_inner);
            *free -= units;
        }
        Lease {
            budget: self,
            units,
        }
    }
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        *self
            .budget
            .free
            .lock()
            .unwrap_or_else(PoisonError::into_inner) += self.units;
        self.budget.freed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use rayon::prelude::*;

    use super::Budget;

    #[test]
    fn parallel_leases_never_exceed_the_budget() {
        let budget = Budget::new(100);
        let held = AtomicU64::new(0);
        let peak = AtomicU64::new(0);
        (0..64_u64).into_par_iter().for_each(|job| {
            let units = 10 + job % 50;
            let _lease = budget.take(units);
            let now = held.fetch_add(units, Ordering::SeqCst) + units;
            peak.fetch_max(now, Ordering::SeqCst);
            held.fetch_sub(units, Ordering::SeqCst);
        });
        assert!(peak.load(Ordering::SeqCst) <= 100, "peak within the budget");
        assert_eq!(*budget.free.lock().expect("lock"), 100, "all given back");
    }

    #[test]
    fn a_lease_larger_than_the_budget_takes_the_whole_budget() {
        let budget = Budget::new(10);
        let lease = budget.take(1_000);
        assert_eq!(lease.units, 10, "capped");
    }
}
