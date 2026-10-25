// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Mutex;

use p4kx_archive::Entry;

use crate::folder_cache::FolderCache;
use crate::{ArchiveWatch, CancelToken, Progress};

/// Counts the progress of one Extraction across worker threads and reports it in order.
pub(crate) struct Tracker<'a> {
    cancel: Option<&'a CancelToken>,
    watch: Option<&'a ArchiveWatch>,
    on_progress: Option<&'a (dyn Fn(Progress) + Sync)>,
    progress: Mutex<Progress>,
    pub(crate) folders: FolderCache,
}

impl<'a> Tracker<'a> {
    pub(crate) fn new(
        entries: &[&Entry],
        cancel: Option<&'a CancelToken>,
        watch: Option<&'a ArchiveWatch>,
        on_progress: Option<&'a (dyn Fn(Progress) + Sync)>,
    ) -> Self {
        Self {
            cancel,
            watch,
            on_progress,
            progress: Mutex::new(Progress {
                entries_total: entries.len(),
                bytes_total: entries.iter().map(|entry| entry.uncompressed_size()).sum(),
                ..Progress::default()
            }),
            folders: FolderCache::default(),
        }
    }

    /// Returns whether the run is to stop: it was cancelled, or the periodic check of the watched
    /// Archive saw a change.
    pub(crate) fn should_stop(&self) -> bool {
        self.cancel.is_some_and(CancelToken::is_cancelled)
            || self.watch.is_some_and(|watch| watch.poll().is_err())
    }

    /// Returns whether the file of the watched Archive differs from when it was opened now or
    /// did at an earlier check.
    pub(crate) fn archive_file_changed(&self) -> bool {
        self.watch.is_some_and(ArchiveWatch::file_changed)
    }

    /// Records `count` bytes written for an Entry that has not ended yet.
    pub(crate) fn wrote(&self, count: u64) {
        self.update(|progress| progress.bytes_done += count);
    }

    /// Takes back `count` bytes recorded through [`wrote`](Self::wrote) that no Entry will
    /// account for when it ends.
    pub(crate) fn retract(&self, count: u64) {
        self.update(|progress| progress.bytes_done = progress.bytes_done.saturating_sub(count));
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
