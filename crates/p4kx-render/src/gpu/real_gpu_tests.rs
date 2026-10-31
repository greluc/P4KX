// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Teardown on the real graphics adapter. Ignored by default: set `P4KX_REAL_GPU_RUNS` to the
//! number of rounds and pass `--ignored`.

use std::time::Instant;

use p4kx_testkit::watchdog::arm;

use super::offscreen_tests::frame;
use super::{Adapters, Offscreen, Released, abandoned_teardowns};

const RUNS: &str = "P4KX_REAL_GPU_RUNS";
const SIZE: [u32; 2] = [96, 64];

fn runs() -> u32 {
    std::env::var(RUNS)
        .ok()
        .and_then(|runs| runs.parse().ok())
        .unwrap_or(1)
}

#[test]
#[ignore = "real adapter: set P4KX_REAL_GPU_RUNS and pass --ignored"]
#[expect(clippy::print_stderr, reason = "reports the timings of a manual run")]
fn closing_the_offscreen_device_on_the_real_adapter_drops_it() {
    let guard = arm("closing_the_offscreen_device_on_the_real_adapter");
    for round in 0..runs() {
        guard.phase("device");
        let mut offscreen = Offscreen::new(Adapters::Best).expect("an adapter");
        assert!(
            !offscreen.is_software(),
            "the adapter {} is a software one, not a real GPU",
            offscreen.adapter_name()
        );
        guard.phase("draw");
        assert!(offscreen.draw(&frame(SIZE)).is_ok(), "round {round} draws");
        let adapter = offscreen.adapter_name().to_owned();
        guard.phase("teardown");
        let started = Instant::now();
        let released = offscreen.close();
        eprintln!(
            "round {round}: {adapter}, {released:?} after {:?}",
            started.elapsed()
        );
        assert_eq!(released, Released::Dropped, "round {round}");
    }
    assert_eq!(abandoned_teardowns(), 0, "no teardown was abandoned");
}
