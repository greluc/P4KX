use std::num::NonZero;
use std::panic;
use std::sync::{Mutex, PoisonError};
use std::thread;

use rayon::prelude::*;

use crate::budget::Budget;
use crate::buffers::Buffers;
use crate::extract::Ended;
use crate::texture_job::Item;

/// The most sound workers of one Extraction.
const MAX_SOUND_WORKERS: usize = 8;
/// The bytes of sounds and soundbanks the sound workers may hold in memory at the same time; a
/// larger one runs alone.
const SOUND_BUDGET: u64 = 1 << 30;

/// Runs `items` in parallel: sounds and soundbanks on their own worker threads within
/// [`SOUND_BUDGET`], every other item on the current rayon pool (ADR-0021).
pub(crate) fn run_parallel<'a, F>(items: Vec<Item<'a>>, run: &F) -> Vec<Ended>
where
    F: Fn(&mut Buffers, Item<'a>) -> Ended + Sync,
{
    let (sounds, others): (Vec<_>, Vec<_>) = items.into_iter().partition(Item::is_sound);
    if sounds.is_empty() {
        return others.into_par_iter().map_init(Buffers::new, run).collect();
    }
    let workers = thread::available_parallelism()
        .map_or(1, NonZero::get)
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
    thread::scope(|scope| {
        let handles: Vec<_> = (1..workers)
            .filter_map(|number| {
                thread::Builder::new()
                    .name(format!("p4kx-sound-{number}"))
                    .spawn_scoped(scope, work)
                    .ok()
            })
            .collect();
        let mut ended: Vec<Ended> = others.into_par_iter().map_init(Buffers::new, run).collect();
        ended.extend(work());
        for handle in handles {
            match handle.join() {
                Ok(done) => ended.extend(done),
                Err(payload) => panic::resume_unwind(payload),
            }
        }
        ended
    })
}
