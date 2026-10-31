// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Bounded teardown of a graphics device: a driver that never answers must not block the caller.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::Renderer;

/// How long the caller waits for the whole teardown: the queue going idle and the drop of the
/// device, both of which run on the worker. A measured teardown takes 40 to 120 ms; the limit
/// leaves room for a loaded machine.
pub(super) const TEARDOWN_LIMIT: Duration = Duration::from_secs(15);

/// The idle wait on the worker, below [`TEARDOWN_LIMIT`] so that an idle wait that runs out is
/// still followed by the drop within the limit.
const IDLE_WAIT: Duration = Duration::from_secs(10);

static ABANDONED: AtomicUsize = AtomicUsize::new(0);

/// Returns how many teardowns of this process were left to a worker thread that had not
/// finished.
#[must_use]
pub fn abandoned_teardowns() -> usize {
    ABANDONED.load(Ordering::Relaxed)
}

/// How a teardown ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Released {
    /// The queue went idle and the device was dropped within the limit.
    Dropped,
    /// The limit passed first; the worker thread keeps waiting on the driver, and the process
    /// does not wait for it when it exits.
    Abandoned,
}

/// Runs `idle` and then drops `value` on a worker thread, and waits at most `limit` for both.
///
/// A worker that panics counts as [`Released::Dropped`]: the value is gone either way.
pub(super) fn release_within<T: Send + 'static>(
    value: T,
    limit: Duration,
    idle: impl FnOnce(&T) + Send + 'static,
) -> Released {
    let (done, finished) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("p4kx-gpu-teardown".to_owned())
        .spawn(move || {
            idle(&value);
            drop(value);
            done.send(()).ok();
        });
    let released = spawned.map_or(Released::Dropped, |_worker| {
        match finished.recv_timeout(limit) {
            Err(mpsc::RecvTimeoutError::Timeout) => Released::Abandoned,
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => Released::Dropped,
        }
    });
    if released == Released::Abandoned {
        ABANDONED.fetch_add(1, Ordering::Relaxed);
    }
    released
}

/// The GPU objects of an [`Offscreen`](super::Offscreen). The fields drop in declaration order:
/// the renderer's buffers and textures go before the queue and the device they were made on.
#[derive(Debug)]
pub(super) struct Gpu {
    pub(super) renderer: Renderer,
    pub(super) queue: wgpu::Queue,
    pub(super) device: wgpu::Device,
}

impl Gpu {
    /// Drops everything on a worker thread after the queue went idle, within
    /// [`TEARDOWN_LIMIT`].
    pub(super) fn release(self) -> Released {
        release_within(self, TEARDOWN_LIMIT, |gpu| {
            drop(gpu.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(IDLE_WAIT),
            }));
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::Sender;
    use std::time::Instant;

    use super::*;

    /// A value whose drop waits until the test lets go, as a driver that never answers would.
    struct Stuck(mpsc::Receiver<()>);

    impl Drop for Stuck {
        fn drop(&mut self) {
            self.0.recv().ok();
        }
    }

    fn stuck() -> (Stuck, Sender<()>) {
        let (release, wait) = mpsc::channel();
        (Stuck(wait), release)
    }

    #[test]
    fn a_value_that_drops_at_once_is_released() {
        let released = release_within(vec![1_u8], Duration::from_secs(5), |_| {});

        assert_eq!(released, Released::Dropped);
    }

    #[test]
    fn the_idle_step_runs_before_the_drop() {
        let (order, seen) = mpsc::channel();
        let sender = order.clone();

        let released = release_within(order, Duration::from_secs(5), move |_| {
            sender.send("idle").ok();
        });

        assert_eq!(released, Released::Dropped);
        assert_eq!(seen.try_recv(), Ok("idle"));
    }

    #[test]
    fn a_drop_that_does_not_return_is_abandoned_at_the_limit_and_counted() {
        let (value, release) = stuck();
        let before = abandoned_teardowns();
        let started = Instant::now();

        let released = release_within(value, Duration::from_millis(100), |_| {});

        assert_eq!(released, Released::Abandoned);
        assert!(started.elapsed() < Duration::from_secs(3), "returned early");
        assert!(abandoned_teardowns() > before, "counted");
        release.send(()).ok();
    }

    #[test]
    fn an_idle_step_that_does_not_return_is_abandoned_too() {
        let (hold, wait) = mpsc::channel::<()>();

        let released = release_within((), Duration::from_millis(100), move |()| {
            wait.recv().ok();
        });

        assert_eq!(released, Released::Abandoned);
        hold.send(()).ok();
    }
}
