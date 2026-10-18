use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_comparison::Filter;
use rayon::prelude::*;

use super::record_list::{RecordChangeList, RecordChangeRow};

/// What became of a running Record filter.
pub(crate) enum Filtered {
    Running,
    Finished(Vec<bool>),
    Lost,
}

/// Matching a Filter against the Records on a background thread; dropping it cancels the thread.
#[derive(Debug)]
pub(crate) struct RecordFilterJob {
    cancelled: Arc<AtomicBool>,
    result: Receiver<Vec<bool>>,
}

impl RecordFilterJob {
    /// Matches `filter` against the record paths of `list` on a background thread, calling
    /// `wake` from it once the mask is ready; the mask is made on the calling thread when no
    /// thread can start.
    pub(crate) fn start(
        list: &Arc<RecordChangeList>,
        filter: &Filter,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, Vec<bool>> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, result) = mpsc::channel();
        let thread_list = Arc::clone(list);
        let thread_filter = filter.clone();
        let thread_cancelled = Arc::clone(&cancelled);
        let spawned = thread::Builder::new()
            .name("comparison-record-filter".to_owned())
            .spawn(move || {
                if let Some(mask) = matching(&thread_list, &thread_filter, &thread_cancelled)
                    && sender.send(mask).is_ok()
                {
                    wake();
                }
            });
        match spawned {
            Ok(_handle) => Ok(Self { cancelled, result }),
            Err(_ignored) => {
                Err(matching(list, filter, &AtomicBool::new(false)).unwrap_or_default())
            }
        }
    }

    /// Takes the mask once the filter finished.
    pub(crate) fn poll(&self) -> Filtered {
        match self.result.try_recv() {
            Ok(mask) => Filtered::Finished(mask),
            Err(TryRecvError::Empty) => Filtered::Running,
            Err(TryRecvError::Disconnected) => Filtered::Lost,
        }
    }
}

impl Drop for RecordFilterJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Returns whether `filter` selects `row` by the record path of either side, as the Record level
/// of the Comparison selects Records; a row without a readable record path stays.
pub(crate) fn selects(filter: &Filter, row: &RecordChangeRow) -> bool {
    let mut paths = [row.old_path(), row.new_path()]
        .into_iter()
        .flatten()
        .peekable();
    paths.peek().is_none() || paths.any(|path| filter.matches(path))
}

fn matching(list: &RecordChangeList, filter: &Filter, cancelled: &AtomicBool) -> Option<Vec<bool>> {
    let mask: Vec<bool> = list
        .rows()
        .par_iter()
        .map(|row| !cancelled.load(Ordering::Relaxed) && selects(filter, row))
        .collect();
    (!cancelled.load(Ordering::Relaxed)).then_some(mask)
}
