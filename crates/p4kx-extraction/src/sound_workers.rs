// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::panic;
use std::sync::{Mutex, PoisonError};
use std::thread;

use rayon::ThreadPool;
use rayon::prelude::*;

use crate::budget::Budget;
use crate::buffers::Buffers;
use crate::extract::Ended;
use crate::texture_job::Item;

/// The most sound workers of one Extraction.
const MAX_SOUND_WORKERS: usize = 8;
/// The bytes of sounds and soundbanks the sound workers may hold in memory at the same time; a
/// larger item takes all of it.
const SOUND_BUDGET: u64 = 1 << 30;

/// Runs `work` on `pool`, or on the calling thread's pool when there is none.
pub(crate) fn within<R: Send>(pool: Option<&ThreadPool>, work: impl FnOnce() -> R + Send) -> R {
    match pool {
        Some(pool) => pool.install(work),
        None => work(),
    }
}

/// Runs `items` in parallel: sounds and soundbanks on worker threads of their own within
/// [`SOUND_BUDGET`], every other item on `pool` or the current rayon pool (ADR-0022).
///
/// The workers wait for the budget on their own threads, never on a pool thread, and run only
/// the conversion of an item on `pool`, so a pool of limited size bounds the compute and a wait
/// cannot block the pool. A cancelled run stops waiting for the budget.
pub(crate) fn run_parallel<'a, F>(
    items: Vec<Item<'a>>,
    run: &F,
    pool: Option<&ThreadPool>,
    stop: &(dyn Fn() -> bool + Sync),
) -> Vec<Ended>
where
    F: Fn(&mut Buffers, Item<'a>) -> Ended + Sync,
{
    let (sounds, others) = items.into_iter().partition::<Vec<_>, _>(Item::is_sound);
    let parallel =
        || -> Vec<Ended> { others.into_par_iter().map_init(Buffers::new, run).collect() };
    if sounds.is_empty() {
        return within(pool, parallel);
    }
    let threads = pool.map_or_else(rayon::current_num_threads, ThreadPool::current_num_threads);
    let workers = threads.clamp(1, MAX_SOUND_WORKERS).min(sounds.len());
    drain(
        sounds,
        Item::first_size,
        SOUND_BUDGET,
        workers,
        Buffers::new,
        |buffers, item| within(pool, || run(buffers, item)),
        stop,
        || within(pool, parallel),
    )
}

/// Hands `queue` to `workers` threads that each take `size` units of a budget of `budget` before
/// they advance an item, and runs `meanwhile` on the calling thread; returns every result.
///
/// An item whose wait for the budget was stopped by `stop` is advanced without a lease, which a
/// stopped run ends at once. A worker thread the system refuses to spawn is not reported: the
/// calling thread drains the queue if none started, so every item still ends.
#[expect(
    clippy::too_many_arguments,
    reason = "the pieces of one fixed pipeline, passed apart so the pipeline can be tested"
)]
fn drain<T: Send, S, R: Send>(
    queue: Vec<T>,
    size: impl Fn(&T) -> u64 + Sync,
    budget: u64,
    workers: usize,
    init: impl Fn() -> S + Sync,
    advance: impl Fn(&mut S, T) -> R + Sync,
    stop: &(dyn Fn() -> bool + Sync),
    meanwhile: impl FnOnce() -> Vec<R>,
) -> Vec<R> {
    let queue = Mutex::new(queue.into_iter());
    let budget = Budget::new(budget);
    let work = || {
        let mut state = init();
        let mut done = Vec::new();
        loop {
            let next = queue.lock().unwrap_or_else(PoisonError::into_inner).next();
            let Some(item) = next else {
                return done;
            };
            let _lease = budget.take_unless(size(&item), stop);
            done.push(advance(&mut state, item));
        }
    };
    thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .filter_map(|number| {
                thread::Builder::new()
                    .name(format!("p4kx-sound-{number}"))
                    .spawn_scoped(scope, work)
                    .ok()
            })
            .collect();
        let mut results = meanwhile();
        if handles.is_empty() {
            results.extend(work());
        }
        for handle in handles {
            match handle.join() {
                Ok(done) => results.extend(done),
                Err(payload) => panic::resume_unwind(payload),
            }
        }
        results
    })
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use rayon::ThreadPoolBuilder;
    use rayon::prelude::*;

    use super::{drain, within};

    #[test]
    fn sounds_that_together_exceed_the_budget_finish_with_nested_work_on_a_small_pool() {
        let pool = ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .expect("pool");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let sizes = vec![60_u64; 24];
            let results = drain(
                sizes,
                |size| *size,
                100,
                4,
                || (),
                |(), size| {
                    within(Some(&pool), || {
                        size + (0..2000_u64).into_par_iter().map(|n| n % 3).sum::<u64>()
                    })
                },
                &|| false,
                Vec::new,
            );
            let _ignored = sender.send(results.len());
        });

        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(60)),
            Ok(24),
            "every item ends"
        );
    }

    #[test]
    fn a_stopped_run_does_not_wait_for_the_budget() {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let results = drain(
                vec![100_u64; 6],
                |size| *size,
                100,
                3,
                || (),
                |(), size| {
                    std::thread::sleep(Duration::from_millis(200));
                    size
                },
                &|| true,
                Vec::new,
            );
            let _ignored = sender.send(results.len());
        });

        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(30)),
            Ok(6),
            "every item is advanced without waiting"
        );
    }
}
