// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Teardown on the real graphics adapter. Ignored by default: set `P4KX_REAL_GPU_RUNS` to the
//! number of rounds and pass `--ignored`.

use std::thread;
use std::time::Instant;

use p4kx_testkit::watchdog::arm;

use super::offscreen_tests::frame;
use super::{Adapters, Offscreen, Released, abandoned_teardowns};

const RUNS: &str = "P4KX_REAL_GPU_RUNS";
const SIZE: [u32; 2] = [96, 64];
const THREADS: usize = 8;

fn runs() -> u32 {
    std::env::var(RUNS)
        .ok()
        .and_then(|runs| runs.parse().ok())
        .unwrap_or(1)
}

/// Opens a device on the best adapter and draws one picture on it.
fn drawn_offscreen() -> Offscreen {
    let mut offscreen = Offscreen::new(Adapters::Best).expect("an adapter");
    assert!(
        !offscreen.is_software(),
        "the adapter {} is a software one, not a real GPU",
        offscreen.adapter_name()
    );
    assert!(offscreen.draw(&frame(SIZE)).is_ok(), "draws");
    offscreen
}

#[test]
#[ignore = "real adapter: set P4KX_REAL_GPU_RUNS and pass --ignored"]
#[expect(clippy::print_stderr, reason = "reports the timings of a manual run")]
fn closing_the_offscreen_device_on_the_real_adapter_drops_it() {
    let guard = arm("closing_the_offscreen_device_on_the_real_adapter");
    for round in 0..runs() {
        guard.phase("device and draw");
        let offscreen = drawn_offscreen();
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

#[test]
#[ignore = "real adapter: set P4KX_REAL_GPU_RUNS and pass --ignored"]
#[expect(
    clippy::needless_collect,
    reason = "every thread must have started before the first is joined"
)]
fn closing_devices_on_several_threads_at_once_drops_them_all() {
    let guard = arm("closing_devices_on_several_threads_at_once");
    guard.phase("threads");
    let released: Vec<Released> = thread::scope(|scope| {
        let workers: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    (0..runs())
                        .map(|_| drawn_offscreen().close())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a worker"))
            .collect()
    });
    assert!(
        released
            .iter()
            .all(|released| *released == Released::Dropped),
        "{released:?}"
    );
    assert_eq!(abandoned_teardowns(), 0, "no teardown was abandoned");
}
