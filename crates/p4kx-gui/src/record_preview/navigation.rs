use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_extraction::Guid;

use super::worker::{Answered, Request, Unread, Worker};
use super::{RecordPreview, State};
use crate::{EntryIndex, RecordId, TextLanguage};

/// The most Records back and forward each keep.
const HISTORY_LIMIT: usize = 256;

static GENERATIONS: AtomicU64 = AtomicU64::new(1);

/// The Records shown in the preview: the current one, the way taken to it and back, its preview
/// and the thread that reads it.
#[derive(Debug, Default)]
pub(crate) struct Navigation {
    back: VecDeque<RecordId>,
    current: Option<RecordId>,
    forward: VecDeque<RecordId>,
    preview: Option<RecordPreview>,
    worker: Option<Worker>,
    dangling: Option<Guid>,
}

impl Navigation {
    /// Returns the preview of the current Record, or `None` before the first.
    pub(crate) const fn preview(&self) -> Option<&RecordPreview> {
        self.preview.as_ref()
    }

    /// Returns the GUID of the missing Record the last followed Reference pointed at, until
    /// another Record is shown.
    pub(crate) const fn dangling(&self) -> Option<Guid> {
        self.dangling
    }

    /// Returns the current Record, or `None` before the first.
    pub(crate) const fn current(&self) -> Option<RecordId> {
        self.current
    }

    /// Returns whether the Record `id` is previewed or read for the preview already, with its
    /// localisation keys resolved in `language`.
    pub(crate) fn previews(&self, id: RecordId, language: Option<TextLanguage>) -> bool {
        self.preview.as_ref().is_some_and(|preview| {
            preview.record == id
                && preview.language == language
                && !matches!(preview.state, State::Failed(_))
        })
    }

    /// Returns whether a Record lies back on the way taken.
    pub(crate) fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Returns whether a Record lies forward on the way taken.
    pub(crate) fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Makes `id` the current Record, the one before it the last step back, and forgets the
    /// way forward; returns whether it was not current already.
    pub(crate) fn visit(&mut self, id: RecordId) -> bool {
        self.dangling = None;
        if self.current == Some(id) {
            return false;
        }
        if let Some(current) = self.current.replace(id) {
            push(&mut self.back, current);
        }
        self.forward.clear();
        true
    }

    /// Notes that a followed Reference points at the missing Record `guid`.
    pub(crate) const fn follow_dangling(&mut self, guid: Guid) {
        self.dangling = Some(guid);
    }

    /// Steps back along the way taken; returns the Record now current.
    pub(crate) fn back(&mut self) -> Option<RecordId> {
        let id = self.back.pop_back()?;
        if let Some(current) = self.current.replace(id) {
            push(&mut self.forward, current);
        }
        self.dangling = None;
        Some(id)
    }

    /// Steps forward along the way taken; returns the Record now current.
    pub(crate) fn forward(&mut self) -> Option<RecordId> {
        let id = self.forward.pop_back()?;
        if let Some(current) = self.current.replace(id) {
            push(&mut self.back, current);
        }
        self.dangling = None;
        Some(id)
    }

    /// Reads the Record `id` of `index` for the preview unless it is previewed already, with its
    /// localisation keys resolved in `language`, starting the preview thread first, which calls
    /// `wake` after each Record it read.
    ///
    /// A Record that is shown already stays on screen while it is read again for another
    /// `language`; the thread reads it from the DataForge it holds open.
    pub(crate) fn show(
        &mut self,
        id: RecordId,
        language: Option<TextLanguage>,
        index: &Arc<EntryIndex>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) {
        if self.previews(id, language) {
            return;
        }
        let generation = GENERATIONS.fetch_add(1, Ordering::Relaxed);
        let request = Request {
            generation,
            id,
            language,
        };
        let mut state = State::Loading;
        let sent = self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.request(request));
        if !sent {
            match Worker::start(index, wake) {
                Ok(worker) => {
                    if !worker.request(request) {
                        state = State::Failed(None);
                    }
                    self.worker = Some(worker);
                }
                Err(reason) => state = State::Failed(Some(reason)),
            }
        }
        if let Some(shown) = &mut self.preview
            && shown.record == id
            && matches!(shown.state, State::Ready(_))
            && !matches!(state, State::Failed(_))
        {
            shown.language = language;
            shown.pending = Some(generation);
            return;
        }
        self.preview = Some(RecordPreview {
            record: id,
            generation,
            language,
            pending: None,
            state,
        });
    }

    /// Takes the Records the preview thread read; returns whether the preview changed.
    pub(crate) fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Some(worker) = &self.worker {
            match worker.poll() {
                Answered::Nothing => break,
                Answered::Answer(answer) => {
                    let (generation, read) = *answer;
                    if let Some(preview) = &mut self.preview
                        && (preview.generation == generation || preview.pending == Some(generation))
                    {
                        preview.generation = generation;
                        preview.pending = None;
                        preview.state = match read {
                            Ok(view) => State::Ready(Box::new(view)),
                            Err(Unread::Failed(reason)) => State::Failed(Some(reason)),
                            Err(Unread::ArchiveChanged) => State::ArchiveChanged,
                        };
                        changed = true;
                    }
                }
                Answered::Gone => {
                    self.worker = None;
                    if let Some(preview) = &mut self.preview
                        && matches!(preview.state, State::Loading)
                    {
                        preview.state = State::Failed(None);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// Forgets the way taken and stops the preview thread, because the Archive changed on disk.
    pub(crate) fn archive_changed(&mut self) {
        self.worker = None;
        self.back.clear();
        self.forward.clear();
        self.current = None;
        self.dangling = None;
        if let Some(preview) = &mut self.preview {
            preview.state = State::ArchiveChanged;
        }
    }
}

fn push(way: &mut VecDeque<RecordId>, id: RecordId) {
    if way.len() >= HISTORY_LIMIT {
        way.pop_front();
    }
    way.push_back(id);
}
