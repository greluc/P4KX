// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Bounded teardown of a graphics device: a driver that never answers must not block the caller.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

use super::Renderer;

/// How long the caller waits for the whole teardown: the queue going idle and the release of
/// every GPU object, all of which run on the worker. A measured teardown takes 40 to 120 ms; the
/// limit leaves room for a loaded machine.
pub(super) const TEARDOWN_LIMIT: Duration = Duration::from_secs(15);

/// The idle wait on the worker, below [`TEARDOWN_LIMIT`] so that an idle wait that runs out is
/// still followed by the release within the limit.
const IDLE_WAIT: Duration = Duration::from_secs(10);

static ABANDONED: AtomicUsize = AtomicUsize::new(0);

/// Returns how many teardowns of this process were left to a worker thread that had not
/// finished.
#[must_use]
pub fn abandoned_teardowns() -> usize {
    ABANDONED.load(Ordering::Relaxed)
}

/// The stages of a teardown, in the order the worker runs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Waiting for the submitted work to finish.
    Idle,
    /// Dropping the buffers, textures and pipelines of the renderer.
    Renderer,
    /// Dropping the queue.
    Queue,
    /// Dropping the device, which destroys it in the driver.
    Device,
}

impl fmt::Display for Stage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Idle => "waiting for the queue to go idle",
            Self::Renderer => "releasing the renderer resources",
            Self::Queue => "dropping the queue",
            Self::Device => "destroying the device",
        })
    }
}

/// The stage a worker is in, readable by the caller that waits for it.
#[derive(Debug, Clone)]
pub(super) struct StageCell(Arc<Mutex<Stage>>);

impl StageCell {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Stage::Idle)))
    }

    /// Records that the worker is now in `stage`.
    pub(super) fn enter(&self, stage: Stage) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = stage;
    }

    fn current(&self) -> Stage {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// How a teardown ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Released {
    /// The queue went idle and the device was released within the limit.
    Dropped,
    /// The limit passed first, while the worker was in the given stage; the worker thread keeps
    /// waiting on the driver, and the process does not wait for it when it exits.
    Abandoned(Stage),
}

/// Runs `work` on a worker thread and waits at most `limit` for it.
///
/// `work` reports the stage it is in through the [`StageCell`]. A worker that panics counts as
/// [`Released::Dropped`]: the value is gone either way.
pub(super) fn release_within<T: Send + 'static>(
    value: T,
    limit: Duration,
    work: impl FnOnce(T, &StageCell) + Send + 'static,
) -> Released {
    let (done, finished) = mpsc::channel();
    let stage = StageCell::new();
    let worker_stage = stage.clone();
    let spawned = thread::Builder::new()
        .name("p4kx-gpu-teardown".to_owned())
        .spawn(move || {
            work(value, &worker_stage);
            done.send(()).ok();
        });
    let released = spawned.map_or(Released::Dropped, |_worker| {
        match finished.recv_timeout(limit) {
            Err(mpsc::RecvTimeoutError::Timeout) => Released::Abandoned(stage.current()),
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => Released::Dropped,
        }
    });
    if matches!(released, Released::Abandoned(_)) {
        ABANDONED.fetch_add(1, Ordering::Relaxed);
    }
    released
}

/// The GPU objects of an [`Offscreen`](super::Offscreen).
#[derive(Debug)]
pub(super) struct Gpu {
    pub(super) renderer: Renderer,
    pub(super) queue: wgpu::Queue,
    pub(super) device: wgpu::Device,
}

impl Gpu {
    /// Releases everything on a worker thread within [`TEARDOWN_LIMIT`]: waits for the queue to
    /// go idle, then drops the renderer's resources before the queue and the device they were
    /// made on.
    pub(super) fn release(self) -> Released {
        release_within(self, TEARDOWN_LIMIT, |gpu, stage| {
            drop(gpu.device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(IDLE_WAIT),
            }));
            let Self {
                renderer,
                queue,
                device,
            } = gpu;
            stage.enter(Stage::Renderer);
            drop(renderer);
            stage.enter(Stage::Queue);
            drop(queue);
            stage.enter(Stage::Device);
            drop(device);
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
        let released = release_within(vec![1_u8], Duration::from_secs(5), |value, _| drop(value));

        assert_eq!(released, Released::Dropped);
    }

    #[test]
    fn the_stage_follows_what_the_work_enters() {
        let (order, seen) = mpsc::channel();

        let released = release_within(order, Duration::from_secs(5), |order, stage| {
            for next in [Stage::Idle, Stage::Renderer, Stage::Queue, Stage::Device] {
                stage.enter(next);
                order.send(stage.current()).ok();
            }
        });

        assert_eq!(released, Released::Dropped);
        assert_eq!(
            seen.try_iter().collect::<Vec<_>>(),
            [Stage::Idle, Stage::Renderer, Stage::Queue, Stage::Device]
        );
    }

    #[test]
    fn a_drop_that_does_not_return_is_abandoned_in_its_stage_and_counted() {
        let (value, release) = stuck();
        let before = abandoned_teardowns();
        let started = Instant::now();

        let released = release_within(value, Duration::from_millis(100), |value, stage| {
            stage.enter(Stage::Device);
            drop(value);
        });

        assert_eq!(released, Released::Abandoned(Stage::Device));
        assert!(started.elapsed() < Duration::from_secs(3), "returned early");
        assert!(abandoned_teardowns() > before, "counted");
        release.send(()).ok();
    }

    #[test]
    fn work_that_never_leaves_the_first_stage_is_abandoned_in_it() {
        let (hold, wait) = mpsc::channel::<()>();

        let released = release_within((), Duration::from_millis(100), move |(), _| {
            wait.recv().ok();
        });

        assert_eq!(released, Released::Abandoned(Stage::Idle));
        hold.send(()).ok();
    }
}
