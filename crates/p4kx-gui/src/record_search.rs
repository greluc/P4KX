use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::Filter;
use rayon::prelude::*;

use crate::RecordId;
use crate::record_index::RecordIndex;

static GENERATIONS: AtomicU64 = AtomicU64::new(1);

/// Wakes the window from a background thread.
pub(crate) type Wake = Arc<dyn Fn() + Send + Sync>;

struct Pending {
    cancelled: Arc<AtomicBool>,
    result: Receiver<Vec<RecordId>>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// The search over the Records of a DataForge: its text, a search running on a background
/// thread and the Records the last finished search matched.
#[derive(Default)]
pub(crate) struct RecordSearch {
    text: String,
    waiting: Option<Wake>,
    pending: Option<Pending>,
    matches: Option<Vec<RecordId>>,
    generation: u64,
}

impl std::fmt::Debug for RecordSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordSearch")
            .field("text", &self.text)
            .field("waiting", &self.waiting.is_some())
            .field("pending", &self.pending.is_some())
            .field("matches", &self.matches.as_ref().map(Vec::len))
            .finish_non_exhaustive()
    }
}

impl RecordSearch {
    /// Returns the text searched for.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// Returns the Records the last finished search matched, in record path order.
    pub(crate) fn matches(&self) -> Option<&[RecordId]> {
        self.matches.as_deref()
    }

    /// Returns whether a search runs or waits for DataForge.
    pub(crate) const fn is_searching(&self) -> bool {
        self.pending.is_some() || self.waiting.is_some()
    }

    /// Returns a number that changes whenever the matches change.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Searches `index` for `text`, replacing a running search; without `index` the search
    /// waits until [`RecordSearch::loaded`] hands one over. Empty text ends the search.
    pub(crate) fn search(&mut self, text: &str, index: Option<&Arc<RecordIndex>>, wake: Wake) {
        text.clone_into(&mut self.text);
        self.pending = None;
        self.waiting = None;
        if text.is_empty() {
            self.set_matches(None);
            return;
        }
        match index {
            Some(index) => self.start(index, wake),
            None => self.waiting = Some(wake),
        }
    }

    /// Starts the search that waited for DataForge, now loaded as `index`.
    pub(crate) fn loaded(&mut self, index: &Arc<RecordIndex>) {
        if let Some(wake) = self.waiting.take() {
            self.start(index, wake);
        }
    }

    /// Ends the search and forgets its text and matches.
    pub(crate) fn clear(&mut self) {
        self.text.clear();
        self.stop();
    }

    /// Ends the search and forgets its matches, keeping the text.
    pub(crate) fn stop(&mut self) {
        self.pending = None;
        self.waiting = None;
        self.set_matches(None);
    }

    /// Takes the result of a finished search; returns whether one finished.
    pub(crate) fn poll(&mut self) -> bool {
        let Some(pending) = &self.pending else {
            return false;
        };
        let matches = match pending.result.try_recv() {
            Ok(matches) => Some(matches),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => None,
        };
        self.pending = None;
        if matches.is_some() {
            self.set_matches(matches);
        }
        true
    }

    fn set_matches(&mut self, matches: Option<Vec<RecordId>>) {
        self.matches = matches;
        self.generation = GENERATIONS.fetch_add(1, Ordering::Relaxed);
    }

    fn start(&mut self, index: &Arc<RecordIndex>, wake: Wake) {
        let filter = Filter::new(&self.text);
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, result) = mpsc::channel();
        let thread_index = Arc::clone(index);
        let thread_filter = filter.clone();
        let thread_cancelled = Arc::clone(&cancelled);
        let spawned = thread::Builder::new()
            .name("record-search".to_owned())
            .spawn(move || {
                if let Some(matches) = matching(&thread_index, &thread_filter, &thread_cancelled)
                    && sender.send(matches).is_ok()
                {
                    wake();
                }
            });
        match spawned {
            Ok(_handle) => self.pending = Some(Pending { cancelled, result }),
            Err(_ignored) => {
                let matches = matching(index, &filter, &AtomicBool::new(false));
                self.set_matches(matches);
            }
        }
    }
}

/// Returns the Records of `index` whose record path or type name matches `filter`, in record
/// path order, or `None` once `cancelled` is set.
fn matching(index: &RecordIndex, filter: &Filter, cancelled: &AtomicBool) -> Option<Vec<RecordId>> {
    let matches = index
        .by_path
        .par_iter()
        .copied()
        .filter(|id| {
            !cancelled.load(Ordering::Relaxed)
                && index.records.get(id.0 as usize).is_some_and(|record| {
                    filter.matches(record.record_path()) || filter.matches(record.type_name())
                })
        })
        .collect();
    (!cancelled.load(Ordering::Relaxed)).then_some(matches)
}
