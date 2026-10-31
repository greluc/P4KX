// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Bounded teardown of a graphics device: a driver that never answers must not block the caller.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::Renderer;

/// How long the queue gets to go idle before the device is dropped.
const IDLE_WAIT: Duration = Duration::from_secs(10);

/// How long the drop of the device may take before it is left to its worker thread.
const DROP_WAIT: Duration = Duration::from_secs(5);

/// How a release ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Released {
    /// The value was dropped within the limit.
    Dropped,
    /// The drop was still running at the limit; its worker thread keeps waiting on the driver.
    Abandoned,
}

/// Drops `value` on a worker thread and waits at most `limit` for it.
///
/// A drop that has not returned by then is left to the worker, which the process does not wait
/// for when it exits.
pub(super) fn release_within<T: Send + 'static>(value: T, limit: Duration) -> Released {
    let (done, finished) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("p4kx-gpu-teardown".to_owned())
        .spawn(move || {
            drop(value);
            done.send(()).ok();
        });
    spawned.map_or(Released::Dropped, |_worker| {
        match finished.recv_timeout(limit) {
            Err(mpsc::RecvTimeoutError::Timeout) => Released::Abandoned,
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => Released::Dropped,
        }
    })
}

/// The GPU objects of an [`Offscreen`](super::Offscreen). The fields drop in declaration order:
/// the renderer's resources first, then the queue, the device last.
#[derive(Debug)]
pub(super) struct Gpu {
    pub(super) renderer: Renderer,
    pub(super) queue: wgpu::Queue,
    pub(super) device: wgpu::Device,
}

impl Gpu {
    /// Waits for the queue to go idle for a bounded time, then drops everything on a worker
    /// thread.
    pub(super) fn release(self) -> Released {
        drop(self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(IDLE_WAIT),
        }));
        release_within(self, DROP_WAIT)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    struct Stuck;

    impl Drop for Stuck {
        fn drop(&mut self) {
            thread::sleep(Duration::from_secs(5));
        }
    }

    #[test]
    fn a_value_that_drops_at_once_is_released() {
        assert_eq!(
            release_within(vec![1_u8], Duration::from_secs(5)),
            Released::Dropped
        );
    }

    #[test]
    fn a_drop_that_does_not_return_is_abandoned_at_the_limit() {
        let started = Instant::now();

        let released = release_within(Stuck, Duration::from_millis(100));

        assert_eq!(released, Released::Abandoned);
        assert!(started.elapsed() < Duration::from_secs(3), "returned early");
    }
}
