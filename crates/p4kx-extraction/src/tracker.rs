use std::sync::Mutex;

use p4kx_archive::Entry;

use crate::{CancelToken, Progress};

/// Counts the progress of one Extraction across worker threads and reports it in order.
pub(crate) struct Tracker<'a> {
    cancel: Option<&'a CancelToken>,
    on_progress: Option<&'a (dyn Fn(Progress) + Sync)>,
    progress: Mutex<Progress>,
}

impl<'a> Tracker<'a> {
    pub(crate) fn new(
        entries: &[&Entry],
        cancel: Option<&'a CancelToken>,
        on_progress: Option<&'a (dyn Fn(Progress) + Sync)>,
    ) -> Self {
        Self {
            cancel,
            on_progress,
            progress: Mutex::new(Progress {
                entries_total: entries.len(),
                bytes_total: entries.iter().map(|entry| entry.uncompressed_size()).sum(),
                ..Progress::default()
            }),
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancel.is_some_and(CancelToken::is_cancelled)
    }

    /// Records `count` bytes written for an Entry that has not ended yet.
    pub(crate) fn wrote(&self, count: u64) {
        self.update(|progress| progress.bytes_done += count);
    }

    /// Records the end of an Entry of which `written` bytes were reported through
    /// [`wrote`](Self::wrote).
    pub(crate) fn ended(&self, entry: &Entry, written: u64) {
        self.update(|progress| {
            progress.bytes_done += entry.uncompressed_size().saturating_sub(written);
            progress.entries_done += 1;
        });
    }

    fn update(&self, change: impl FnOnce(&mut Progress)) {
        let Ok(mut progress) = self.progress.lock() else {
            return;
        };
        change(&mut progress);
        if let Some(on_progress) = self.on_progress {
            on_progress(*progress);
        }
    }
}
