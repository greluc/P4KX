use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::Filter;

use crate::EntryIndex;

/// The Entries a Filter matched, indexed by id; by default none.
#[derive(Debug, Default)]
pub(crate) struct Matches {
    mask: Vec<bool>,
}

impl Matches {
    /// Returns, indexed by id, whether each Entry matched.
    pub(crate) fn mask(&self) -> &[bool] {
        &self.mask
    }

    /// Returns whether the Entry `id` matched.
    pub(crate) fn contains(&self, id: u32) -> bool {
        self.mask.get(id as usize).copied().unwrap_or(false)
    }
}

/// A search running on a background thread; dropping it cancels the thread.
#[derive(Debug)]
pub(crate) struct Pending {
    cancelled: Arc<AtomicBool>,
    result: Receiver<Vec<bool>>,
}

/// A search that was started.
pub(crate) enum Started {
    /// It runs on a background thread.
    Running(Pending),
    /// It already finished.
    Finished(Matches),
}

/// What became of a running search.
pub(crate) enum Outcome {
    /// It is still running.
    Running,
    /// It finished.
    Finished(Matches),
    /// Its thread ended without a result.
    Lost,
}

impl Pending {
    /// Takes the result once the search finished.
    pub(crate) fn poll(&self) -> Outcome {
        match self.result.try_recv() {
            Ok(mask) => Outcome::Finished(Matches { mask }),
            Err(TryRecvError::Empty) => Outcome::Running,
            Err(TryRecvError::Disconnected) => Outcome::Lost,
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Matches `filter` against every Entry of `index` on a background thread, calling `wake` from
/// it once the result is ready; matches on the calling thread when no thread can be started.
pub(crate) fn start(
    index: &Arc<EntryIndex>,
    filter: &Filter,
    wake: impl Fn() + Send + 'static,
) -> Started {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, result) = mpsc::channel();
    let thread_index = Arc::clone(index);
    let thread_filter = filter.clone();
    let thread_cancelled = Arc::clone(&cancelled);
    let spawned = thread::Builder::new()
        .name("search".to_owned())
        .spawn(move || {
            if let Some(mask) = thread_index.matching(&thread_filter, &thread_cancelled)
                && sender.send(mask).is_ok()
            {
                wake();
            }
        });
    match spawned {
        Ok(_handle) => Started::Running(Pending { cancelled, result }),
        Err(_ignored) => {
            let mask = index
                .matching(filter, &AtomicBool::new(false))
                .unwrap_or_default();
            Started::Finished(Matches { mask })
        }
    }
}
