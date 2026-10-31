// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Teardown on the real graphics adapter. Ignored by default: set `P4KX_REAL_GPU_RUNS` to the
//! number of rounds and pass `--ignored`.

use std::time::{Duration, Instant};

use p4kx_testkit::watchdog::arm;

use super::offscreen_tests::frame;
use super::{Adapters, Offscreen};

const RUNS: &str = "P4KX_REAL_GPU_RUNS";
const SIZE: [u32; 2] = [96, 64];
const LIMIT: Duration = Duration::from_secs(20);

fn runs() -> u32 {
    std::env::var(RUNS)
        .ok()
        .and_then(|runs| runs.parse().ok())
        .unwrap_or(1)
}

#[test]
#[ignore = "real adapter: set P4KX_REAL_GPU_RUNS and pass --ignored"]
#[expect(clippy::print_stderr, reason = "reports the timings of a manual run")]
fn dropping_the_offscreen_device_on_the_real_adapter_returns_within_the_limit() {
    let guard = arm("dropping_the_offscreen_device_on_the_real_adapter");
    for round in 0..runs() {
        guard.phase("device");
        let mut offscreen = Offscreen::new(Adapters::Best).expect("a real adapter");
        guard.phase("draw");
        assert!(offscreen.draw(&frame(SIZE)).is_ok(), "round {round} draws");
        let adapter = offscreen.adapter_name().to_owned();
        guard.phase("teardown");
        let started = Instant::now();
        drop(offscreen);
        let took = started.elapsed();
        eprintln!("round {round}: {adapter}, teardown {took:?}");
        assert!(took < LIMIT, "round {round}: teardown took {took:?}");
    }
}
