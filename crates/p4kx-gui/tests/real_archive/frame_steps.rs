// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Stepping a harness while recording the time of each frame.

use std::time::{Duration, Instant};

use egui_kittest::Harness;
use p4kx_gui::Model;

/// Runs one frame of `harness` and appends its duration to `frames`.
pub(crate) fn step_into(frames: &mut Vec<Duration>, harness: &mut Harness<'_, Model>) {
    let started = Instant::now();
    harness.step();
    frames.push(started.elapsed());
}
