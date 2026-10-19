use std::sync::{Condvar, Mutex, PoisonError};

/// Pixels that parallel jobs may hold decoded at the same time; a job waits until enough are free.
#[derive(Debug)]
pub(crate) struct PixelBudget {
    free: Mutex<u64>,
    freed: Condvar,
    total: u64,
}

/// Pixels taken from a [`PixelBudget`], given back when dropped.
#[derive(Debug)]
pub(crate) struct Lease<'a> {
    budget: &'a PixelBudget,
    pixels: u64,
}

impl PixelBudget {
    /// Creates a budget of `total` pixels.
    pub(crate) const fn new(total: u64) -> Self {
        Self {
            free: Mutex::new(total),
            freed: Condvar::new(),
            total,
        }
    }

    /// Waits until `pixels`, at most the whole budget, are free and takes them.
    pub(crate) fn take(&self, pixels: u64) -> Lease<'_> {
        let pixels = pixels.min(self.total);
        {
            let free = self.free.lock().unwrap_or_else(PoisonError::into_inner);
            let mut free = self
                .freed
                .wait_while(free, |free| *free < pixels)
                .unwrap_or_else(PoisonError::into_inner);
            *free -= pixels;
        }
        Lease {
            budget: self,
            pixels,
        }
    }
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        *self
            .budget
            .free
            .lock()
            .unwrap_or_else(PoisonError::into_inner) += self.pixels;
        self.budget.freed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use rayon::prelude::*;

    use super::PixelBudget;

    #[test]
    fn parallel_leases_never_exceed_the_budget() {
        let budget = PixelBudget::new(100);
        let held = AtomicU64::new(0);
        let peak = AtomicU64::new(0);
        (0..64_u64).into_par_iter().for_each(|job| {
            let pixels = 10 + job % 50;
            let _lease = budget.take(pixels);
            let now = held.fetch_add(pixels, Ordering::SeqCst) + pixels;
            peak.fetch_max(now, Ordering::SeqCst);
            held.fetch_sub(pixels, Ordering::SeqCst);
        });
        assert!(peak.load(Ordering::SeqCst) <= 100, "peak within the budget");
        assert_eq!(*budget.free.lock().expect("lock"), 100, "all given back");
    }

    #[test]
    fn a_lease_larger_than_the_budget_takes_the_whole_budget() {
        let budget = PixelBudget::new(10);
        let lease = budget.take(1_000);
        assert_eq!(lease.pixels, 10, "capped");
    }
}
