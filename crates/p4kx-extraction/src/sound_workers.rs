// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Mutex, PoisonError};

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

/// Runs `items` in parallel on the current rayon pool: sounds and soundbanks on up to
/// [`MAX_SOUND_WORKERS`] of its threads within [`SOUND_BUDGET`], every other item as parallel
/// tasks (ADR-0022).
///
/// The sound workers are tasks of the pool, so the work nested in them stays on it too and a pool
/// of limited size bounds the whole run. The calling thread also drains the sound queue, so every
/// item ends even on a pool of one thread.
pub(crate) fn run_parallel<'a, F>(items: Vec<Item<'a>>, run: &F) -> Vec<Ended>
where
    F: Fn(&mut Buffers, Item<'a>) -> Ended + Sync,
{
    let (sounds, others): (Vec<_>, Vec<_>) = items.into_iter().partition(Item::is_sound);
    if sounds.is_empty() {
        return others.into_par_iter().map_init(Buffers::new, run).collect();
    }
    let workers = rayon::current_num_threads()
        .clamp(1, MAX_SOUND_WORKERS)
        .min(sounds.len());
    let queue = Mutex::new(sounds.into_iter());
    let budget = Budget::new(SOUND_BUDGET);
    let work = || {
        let mut buffers = Buffers::new();
        let mut ended = Vec::new();
        loop {
            let next = queue.lock().unwrap_or_else(PoisonError::into_inner).next();
            let Some(item) = next else {
                return ended;
            };
            let _lease = budget.take(item.first_size());
            ended.push(run(&mut buffers, item));
        }
    };
    let finished = Mutex::new(Vec::new());
    let mut ended = rayon::scope(|scope| {
        for _ in 1..workers {
            scope.spawn(|_| {
                let done = work();
                finished
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .extend(done);
            });
        }
        let mut ended: Vec<Ended> = others.into_par_iter().map_init(Buffers::new, run).collect();
        ended.extend(work());
        ended
    });
    ended.extend(
        finished
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner),
    );
    ended
}
