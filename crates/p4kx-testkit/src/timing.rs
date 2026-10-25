// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::Write;
use std::time::{Duration, Instant};

/// The wall-clock times of the repeated runs of one measured operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    runs: Vec<Duration>,
}

impl Timing {
    /// Returns the fastest run.
    #[must_use]
    pub fn min(&self) -> Duration {
        self.runs.first().copied().unwrap_or_default()
    }

    /// Returns the median run.
    #[must_use]
    pub fn median(&self) -> Duration {
        self.runs
            .get(self.runs.len() / 2)
            .copied()
            .unwrap_or_default()
    }

    /// Returns how many runs were timed.
    #[must_use]
    pub const fn runs(&self) -> usize {
        self.runs.len()
    }
}

/// Times `runs` calls of `work` after one untimed warm-up and returns the timing.
///
/// Writes `BENCH <label>: median <ms> ms, min <ms> ms over <runs> runs` to stderr; the result of
/// `work` is kept alive until the clock stops.
pub fn bench<T>(label: &str, runs: usize, mut work: impl FnMut() -> T) -> Timing {
    drop(work());
    let mut times: Vec<Duration> = (0..runs.max(1))
        .map(|_| {
            let started = Instant::now();
            let result = work();
            let elapsed = started.elapsed();
            drop(result);
            elapsed
        })
        .collect();
    times.sort_unstable();
    let timing = Timing { runs: times };
    let _ignored = writeln!(
        std::io::stderr().lock(),
        "BENCH {label}: median {:.3} ms, min {:.3} ms over {} runs",
        timing.median().as_secs_f64() * 1e3,
        timing.min().as_secs_f64() * 1e3,
        timing.runs()
    );
    timing
}

/// Times one call of `work`, which is too slow or too destructive to repeat.
///
/// Writes `BENCH <label>: <ms> ms` to stderr and returns the result with its duration.
pub fn bench_once<T>(label: &str, work: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let result = work();
    let elapsed = started.elapsed();
    let _ignored = writeln!(
        std::io::stderr().lock(),
        "BENCH {label}: {:.3} ms",
        elapsed.as_secs_f64() * 1e3
    );
    (result, elapsed)
}
