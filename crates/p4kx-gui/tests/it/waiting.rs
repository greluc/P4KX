// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The one way the headless window tests wait for asynchronous work: step the window until a
//! condition holds, within a time limit, and fail with the name of what was waited for.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use p4kx_gui::Model;

/// How long a window may take to reach a state; generous because a software renderer shares the
/// machine with every other test.
pub(crate) const PATIENCE: Duration = Duration::from_secs(60);

const PAUSE: Duration = Duration::from_millis(2);

/// Steps `harness` until `done` holds for it, pausing between the frames, and fails with a message
/// naming `what` once [`PATIENCE`] passed. `done` is checked after a frame and again before the
/// time runs out, so a condition that holds at once costs one frame.
pub(crate) fn wait_for<S>(
    harness: &mut Harness<'_, S>,
    what: &str,
    mut done: impl FnMut(&Harness<'_, S>) -> bool,
) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    harness.step();
    while !done(harness) {
        if started.elapsed() > PATIENCE {
            return Err(format!("timed out after {PATIENCE:?} waiting for {what}").into());
        }
        std::thread::sleep(PAUSE);
        harness.step();
    }
    Ok(())
}

/// Steps a window of the [`Model`] until `done` holds for the model.
pub(crate) fn wait_for_model(
    harness: &mut Harness<'_, Model>,
    what: &str,
    mut done: impl FnMut(&Model) -> bool,
) -> Result<(), Box<dyn Error>> {
    wait_for(harness, what, |harness| done(harness.state()))
}

/// Runs a window to rest.
pub(crate) trait RunQuiet {
    /// Steps the window until it asked for no more frames, and panics once [`PATIENCE`] passed.
    /// Unlike `Harness::run`, a window that keeps asking for frames while work finishes in the
    /// background does not fail on a step count.
    fn run_quiet(&mut self);
}

impl<S> RunQuiet for Harness<'_, S> {
    #[track_caller]
    fn run_quiet(&mut self) {
        let started = Instant::now();
        while self.run_ok().is_none() {
            assert!(
                started.elapsed() <= PATIENCE,
                "the window kept asking for frames for {PATIENCE:?}"
            );
            std::thread::sleep(PAUSE);
        }
    }
}
