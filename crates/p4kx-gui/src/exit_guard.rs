// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A bound on the window's shutdown: the graphics device is dropped by eframe after the app's
//! exit hook, and a driver that never answers there would keep the process alive.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// How long the shutdown after the exit hook may take before the process ends. A normal
/// shutdown takes well under a second.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(10);

/// The exit status of a process the guard ended, distinct from a clean exit (0) and from a
/// failed start (1).
pub(crate) const EXIT_STATUS: i32 = 4;

/// The file in the settings directory that records that the guard ended the process.
const NOTE_FILE: &str = "exit-guard.txt";

const MESSAGE: &str = "the graphics driver did not finish shutting down; P4KX ended the process";

/// Ends the process with [`EXIT_STATUS`] when the shutdown is still running after the limit,
/// after a line on stderr and a note in `settings_dir`.
///
/// The guard is never disarmed: a shutdown that finishes ends the process, and the thread with
/// it. The app has saved its settings before this is called. A catalog write that is still
/// running may leave its temporary file behind. On Windows the exit runs the detach routines of
/// the loaded DLLs, among them the suspected driver; `TerminateProcess` would avoid that but
/// needs `unsafe` code, which ADR-0006 does not allow here.
pub(crate) fn arm(settings_dir: Option<PathBuf>) {
    arm_with(SHUTDOWN_LIMIT, move || {
        fire(settings_dir.as_deref());
        std::process::exit(EXIT_STATUS);
    });
}

/// Writes the line and the note.
fn fire(settings_dir: Option<&Path>) {
    writeln!(std::io::stderr().lock(), "{MESSAGE}").ok();
    if let Some(dir) = settings_dir {
        std::fs::write(dir.join(NOTE_FILE), format!("{MESSAGE}\n")).ok();
    }
}

/// Runs `action` on a detached thread after `limit`.
fn arm_with(limit: Duration, action: impl FnOnce() + Send + 'static) {
    let spawned = thread::Builder::new()
        .name("p4kx-exit-guard".to_owned())
        .spawn(move || {
            thread::sleep(limit);
            action();
        });
    if let Err(error) = spawned {
        writeln!(
            std::io::stderr().lock(),
            "the exit guard could not start: {error}"
        )
        .ok();
    }
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
    fn the_action_does_not_run_before_the_limit() {
        let (sender, receiver) = mpsc::channel();
        let (hold, wait) = mpsc::channel::<()>();

        arm_with(Duration::from_millis(500), move || {
            wait.recv().ok();
            sender.send(()).ok();
        });

        assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
        hold.send(()).ok();
    }

    #[test]
    fn firing_leaves_a_note_in_the_settings_directory() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        fire(Some(dir.path()));

        let note = std::fs::read_to_string(dir.path().join(NOTE_FILE)).expect("the note");
        assert!(note.contains("graphics driver"), "{note}");
    }
}
