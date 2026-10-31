// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A watchdog for test processes: a stuck graphics driver ends the run with a message naming the
//! test and its phase instead of hanging it. Exit code [`EXIT_CODE`].

use std::io::Write;
use std::process;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// The environment variable that overrides [`PROCESS_LIMIT`], in whole seconds.
pub const PROCESS_LIMIT_VAR: &str = "P4KX_TEST_DEADLINE_SECS";

/// The exit code of a test process the watchdog ended.
pub const EXIT_CODE: i32 = 3;

/// How long a whole test process may run before the watchdog ends it.
pub const PROCESS_LIMIT: Duration = Duration::from_secs(600);

/// How long one armed test may run before the watchdog ends the process.
pub const TEST_LIMIT: Duration = Duration::from_secs(120);

/// How long the process gets to exit by itself before it is aborted.
const EXIT_GRACE: Duration = Duration::from_secs(5);

struct Running {
    id: u64,
    test: String,
    phase: &'static str,
    since: Instant,
}

struct Registry {
    next_id: u64,
    running: Vec<Running>,
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry {
    next_id: 0,
    running: Vec::new(),
});
static PROCESS: OnceLock<()> = OnceLock::new();

fn registry() -> std::sync::MutexGuard<'static, Registry> {
    REGISTRY.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Describes every armed test that is still running and the phase it reported last.
fn running_report() -> String {
    let registry = registry();
    if registry.running.is_empty() {
        return "no armed test is running (the process hangs outside a test, e.g. while exiting)"
            .to_owned();
    }
    registry
        .running
        .iter()
        .map(|entry| {
            format!(
                "  {} in phase \"{}\" for {} s",
                entry.test,
                entry.phase,
                entry.since.elapsed().as_secs()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ends the test process with a message: a normal exit first, and an abort when the exit itself
/// blocks, as the exit handlers of a graphics driver can.
fn terminate(reason: &str) -> ! {
    let _ignored = writeln!(
        std::io::stderr().lock(),
        "{reason}\nrunning tests:\n{}",
        running_report()
    );
    thread::spawn(|| {
        thread::sleep(EXIT_GRACE);
        process::abort();
    });
    process::exit(EXIT_CODE)
}

fn process_limit() -> Duration {
    std::env::var(PROCESS_LIMIT_VAR)
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(PROCESS_LIMIT, Duration::from_secs)
}

/// Starts the process watchdog once; [`arm`] calls it.
///
/// A detached thread that is never cancelled ends the process when it still runs after
/// [`PROCESS_LIMIT`], whatever it is doing, also after the last test.
pub fn start() {
    PROCESS.get_or_init(|| {
        let limit = process_limit();
        thread::spawn(move || {
            thread::sleep(limit);
            terminate(&format!(
                "the test process is still running after {} seconds, a graphics driver is hung or the machine is overloaded (limit: {PROCESS_LIMIT_VAR})",
                limit.as_secs()
            ));
        });
    });
}

/// Proof that a test is armed; the test is disarmed when it is dropped.
#[must_use = "the test is disarmed as soon as the guard is dropped"]
pub struct Guard {
    id: u64,
    _cancel: Sender<()>,
}

impl Guard {
    /// Records the phase the test is in, named in the message when the watchdog ends the process.
    pub fn phase(&self, phase: &'static str) {
        if let Some(entry) = registry()
            .running
            .iter_mut()
            .find(|entry| entry.id == self.id)
        {
            entry.phase = phase;
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        registry().running.retain(|entry| entry.id != self.id);
    }
}

/// Arms `test`: starts the process watchdog, registers the test and ends the process with a
/// message naming it and its phase when it is still running after [`TEST_LIMIT`]. Hold the guard
/// for the whole test.
pub fn arm(test: &str) -> Guard {
    start();
    let id = {
        let mut registry = registry();
        registry.next_id += 1;
        let id = registry.next_id;
        registry.running.push(Running {
            id,
            test: test.to_owned(),
            phase: "start",
            since: Instant::now(),
        });
        id
    };
    let (cancel, cancelled) = mpsc::channel::<()>();
    let name = test.to_owned();
    thread::spawn(move || {
        if cancelled.recv_timeout(TEST_LIMIT) == Err(RecvTimeoutError::Timeout) {
            terminate(&format!(
                "{name}: still running after {} seconds, a graphics driver is hung or the machine is overloaded",
                TEST_LIMIT.as_secs()
            ));
        }
    });
    Guard {
        id,
        _cancel: cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_armed_test_is_reported_with_its_phase_until_the_guard_drops() {
        let guard = arm("watchdog_report_probe");
        guard.phase("readback");

        let report = running_report();
        assert!(
            report.contains("watchdog_report_probe in phase \"readback\""),
            "{report}"
        );

        drop(guard);
        assert!(!running_report().contains("watchdog_report_probe"));
    }
}
