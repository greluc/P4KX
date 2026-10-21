use rayon::prelude::*;

use crate::{Found, SearchStats};

const WAVE_TASKS_PER_THREAD: usize = 2;

/// What one task found: its hits in order and what it scanned.
#[derive(Debug)]
pub(crate) struct Chunk<T> {
    pub(crate) hits: Vec<T>,
    pub(crate) stats: SearchStats,
}

impl<T> Default for Chunk<T> {
    fn default() -> Self {
        Self {
            hits: Vec::new(),
            stats: SearchStats::default(),
        }
    }
}

/// Runs `tasks` tasks in parallel waves and appends their hits in task order until more than
/// `limit` exist, so that the hits and the counts depend on the data alone, not on the threads.
///
/// Returns `None` once `stop` has said so. A task that finds more than `limit` hits may stop at
/// `limit + 1`; the counts of the tasks after the one that fills the limit are dropped.
pub(crate) fn collect<T: Send>(
    tasks: usize,
    limit: usize,
    stop: &(dyn Fn() -> bool + Sync),
    task: impl Fn(usize) -> Chunk<T> + Sync,
) -> Option<Found<T>> {
    let wave = rayon::current_num_threads()
        .saturating_mul(WAVE_TASKS_PER_THREAD)
        .max(1);
    let mut found = Found {
        hits: Vec::new(),
        has_more: false,
        stats: SearchStats::default(),
    };
    let mut next = 0;
    while next < tasks && !found.has_more {
        if stop() {
            return None;
        }
        let end = tasks.min(next.saturating_add(wave));
        let chunks: Vec<Chunk<T>> = (next..end).into_par_iter().map(&task).collect();
        if stop() {
            return None;
        }
        for chunk in chunks {
            found.stats.merge(&chunk.stats);
            found.hits.extend(chunk.hits);
            if found.hits.len() > limit {
                found.hits.truncate(limit);
                found.has_more = true;
                break;
            }
        }
        next = end;
    }
    Some(found)
}
