// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A bound on the window's shutdown: the graphics device is dropped by eframe after the app's
//! exit hook, and a driver that never answers there would keep the process alive.

use std::thread;
use std::time::Duration;

/// How long the shutdown after the exit hook may take before the process ends.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(10);

/// Ends the process with status 0 when the shutdown is still running after [`SHUTDOWN_LIMIT`].
/// Everything the app keeps has been saved by the exit hook before this is called.
pub(crate) fn arm() {
    arm_with(SHUTDOWN_LIMIT, || std::process::exit(0));
}

/// Runs `action` on a detached thread after `limit`. A process that exits first never runs it.
fn arm_with(limit: Duration, action: impl FnOnce() + Send + 'static) {
    let spawned = thread::Builder::new()
        .name("p4kx-exit-guard".to_owned())
        .spawn(move || {
            thread::sleep(limit);
            action();
        });
    drop(spawned);
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn the_action_runs_after_the_limit() {
        let (sender, receiver) = mpsc::channel();

        arm_with(Duration::from_millis(20), move || {
            sender.send(()).ok();
        });

        assert!(receiver.recv_timeout(Duration::from_secs(10)).is_ok());
    }

    #[test]
    fn the_action_waits_for_the_limit() {
        let (sender, receiver) = mpsc::channel();

        arm_with(Duration::from_secs(60), move || {
            sender.send(()).ok();
        });

        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
    }
}
