// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::Write;
use std::time::{Duration, Instant};

/// The environment variable that switches the `bench_*` tests on; without it, or a test filter
/// that starts with `bench_`, they return at once.
pub const BENCH_VAR: &str = "P4KX_BENCH";

/// Returns whether the `bench_*` tests are switched on: [`BENCH_VAR`] is `1`, or a command-line
/// argument of the test binary is a filter starting with `bench_`. Otherwise writes a line saying
/// so to stderr.
#[must_use]
pub fn bench_enabled() -> bool {
    let enabled = std::env::var_os(BENCH_VAR).is_some_and(|value| value == "1")
        || std::env::args()
            .skip(1)
            .any(|argument| argument.starts_with("bench_"));
    if !enabled {
        let _ignored = writeln!(
            std::io::stderr().lock(),
            "skipped: set {BENCH_VAR}=1 or name a bench_ filter to run the timing tests"
        );
    }
    enabled
}

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
