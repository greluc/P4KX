// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cell::RefCell;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

const TEST_LIMIT: Duration = Duration::from_secs(120);

thread_local! {
    static ARMED: RefCell<Option<Deadline>> = const { RefCell::new(None) };
}

/// A watchdog that runs `on_expiry` once `limit` has passed, unless it is dropped first.
pub(super) struct Deadline {
    _cancel: Sender<()>,
}

impl Deadline {
    pub(super) fn after(limit: Duration, on_expiry: impl FnOnce() + Send + 'static) -> Self {
        let (cancel, expired) = mpsc::channel::<()>();
        thread::spawn(move || {
            if expired.recv_timeout(limit) == Err(RecvTimeoutError::Timeout) {
                on_expiry();
            }
        });
        Self { _cancel: cancel }
    }
}

/// Starts the deadline of the test running on this thread: when it is still running after two
/// minutes, the whole test run is aborted with a message naming `test`. A draw on a hung driver
/// never returns, and `cargo test` has no time limit of its own. The run is aborted rather than
/// exited, because the exit handlers of a graphics driver can block on the very call that hung.
/// The deadline ends with the thread, or when the next one is armed on it.
#[expect(
    clippy::print_stderr,
    reason = "names the test that hit its deadline before the run ends"
)]
pub(super) fn arm(test: &str) {
    let test = test.to_owned();
    let deadline = Deadline::after(TEST_LIMIT, move || {
        eprintln!(
            "{test}: still running after {} seconds, the graphics driver is hung or the machine is overloaded",
            TEST_LIMIT.as_secs()
        );
        std::process::abort();
    });
    ARMED.with(|armed| *armed.borrow_mut() = Some(deadline));
}
