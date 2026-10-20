use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use p4kx_extraction::{DataForge, ReferenceIndex};

use super::{FieldLine, LineValue, RecordLink};
use crate::forge_cache::ReadFailure;
use crate::model::describe;
use crate::{EntryIndex, RecordId};

/// The most References the "Referenced by" list shows; the count line still counts all of them.
pub const MAX_INCOMING_LINES: usize = 10_000;

static GENERATIONS: AtomicU64 = AtomicU64::new(1);

/// The References to one Record as the "Referenced by" list shows them: one line per Reference
/// with the field that holds it and the Record that holds it as a link.
#[derive(Debug)]
pub struct IncomingView {
    lines: Vec<FieldLine>,
    references: usize,
    records: usize,
    incomplete: bool,
}

impl IncomingView {
    /// Returns the lines in the order of the referring Records and of the fields in each; the
    /// field is the name of a line, the referring Record its [`LineValue::Reference`].
    #[must_use]
    pub fn lines(&self) -> &[FieldLine] {
        &self.lines
    }

    /// Returns the number of References to the Record, also those past
    /// [`MAX_INCOMING_LINES`] that have no line.
    #[must_use]
    pub const fn references(&self) -> usize {
        self.references
    }

    /// Returns the number of Records that hold at least one of the References.
    #[must_use]
    pub const fn records(&self) -> usize {
        self.records
    }

    /// Returns whether the list shows fewer lines than there are References.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.lines.len() < self.references
    }

    /// Returns whether some Records could not be indexed or the index is full, so that the list
    /// may miss References.
    #[must_use]
    pub const fn is_incomplete(&self) -> bool {
        self.incomplete
    }
}

/// Where finding the Records that refer to the previewed Record stands.
#[derive(Debug, Clone, Copy)]
pub enum IncomingStatus<'a> {
    /// The Records are found off the UI thread.
    Loading,
    /// The Records were found.
    Ready(&'a IncomingView),
    /// They cannot be found, for the reason given, or finding stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so they were not found.
    ArchiveChanged,
}

#[derive(Debug)]
enum State {
    Loading,
    Ready(Box<IncomingView>),
    Failed(Option<String>),
    ArchiveChanged,
}

/// The "Referenced by" list of one Record.
#[derive(Debug)]
pub struct IncomingPreview {
    record: RecordId,
    generation: u64,
    state: State,
}

impl IncomingPreview {
    /// Returns the Record whose referrers are listed.
    #[must_use]
    pub const fn record(&self) -> RecordId {
        self.record
    }

    /// Returns a number no other list of this process has, for caches.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns where the list stands.
    #[must_use]
    pub fn status(&self) -> IncomingStatus<'_> {
        match &self.state {
            State::Loading => IncomingStatus::Loading,
            State::Ready(view) => IncomingStatus::Ready(view),
            State::Failed(reason) => IncomingStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => IncomingStatus::ArchiveChanged,
        }
    }
}

enum Unread {
    Failed(String),
    ArchiveChanged,
}

type Answer = (u64, Result<IncomingView, Unread>);

/// The list for the selected Record and the threads that find it; the index of References is
/// built once per Archive by the first of them.
#[derive(Debug)]
pub(crate) struct IncomingList {
    preview: Option<IncomingPreview>,
    answers: Sender<Answer>,
    arrivals: Receiver<Answer>,
}

impl Default for IncomingList {
    fn default() -> Self {
        let (answers, arrivals) = mpsc::channel();
        Self {
            preview: None,
            answers,
            arrivals,
        }
    }
}

impl IncomingList {
    /// Returns the list of the last Record asked for.
    pub(crate) const fn preview(&self) -> Option<&IncomingPreview> {
        self.preview.as_ref()
    }

    /// Finds the Records that refer to `id` on a background thread unless that is done or running,
    /// calling `wake` from it once it is done.
    pub(crate) fn request(
        &mut self,
        id: RecordId,
        index: &Arc<EntryIndex>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) {
        if self
            .preview
            .as_ref()
            .is_some_and(|shown| shown.record == id && !matches!(shown.state, State::Failed(_)))
        {
            return;
        }
        let generation = GENERATIONS.fetch_add(1, Ordering::Relaxed);
        let mut preview = IncomingPreview {
            record: id,
            generation,
            state: State::Loading,
        };
        let answers = self.answers.clone();
        let index = Arc::clone(index);
        let spawned = thread::Builder::new()
            .name("record-incoming".to_owned())
            .spawn(move || {
                let found = find(&index, id);
                if answers.send((generation, found)).is_ok() {
                    wake();
                }
            });
        if let Err(error) = spawned {
            preview.state = State::Failed(Some(describe(&error)));
        }
        self.preview = Some(preview);
    }

    /// Takes the lists the threads found; returns whether the list shown changed.
    pub(crate) fn poll(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.arrivals.try_recv() {
                Ok((generation, found)) => {
                    if let Some(preview) = &mut self.preview
                        && preview.generation == generation
                    {
                        preview.state = match found {
                            Ok(view) => State::Ready(Box::new(view)),
                            Err(Unread::Failed(reason)) => State::Failed(Some(reason)),
                            Err(Unread::ArchiveChanged) => State::ArchiveChanged,
                        };
                        changed = true;
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return changed,
            }
        }
    }

    /// Drops the list, because the Archive changed on disk.
    pub(crate) fn archive_changed(&mut self) {
        if let Some(preview) = &mut self.preview {
            preview.state = State::ArchiveChanged;
        }
    }
}

fn find(index: &EntryIndex, id: RecordId) -> Result<IncomingView, Unread> {
    let unread = |failure: ReadFailure| match failure {
        ReadFailure::ArchiveChanged(_) => Unread::ArchiveChanged,
        ReadFailure::Unreadable(reason) => Unread::Failed(reason),
    };
    let source = index.forge().read(index, || {}).map_err(unread)?;
    let references = source.references().map_err(unread)?;
    let forge = source
        .entry
        .open()
        .map_err(|error| Unread::Failed(describe(&error)))?;
    let view = view(&forge, &references, id);
    if index.watch().check().is_err() {
        return Err(Unread::ArchiveChanged);
    }
    Ok(view)
}

fn view(forge: &DataForge<'_>, references: &ReferenceIndex, id: RecordId) -> IncomingView {
    let total = references.incoming(id.0).len();
    let mut lines = Vec::with_capacity(total.min(MAX_INCOMING_LINES));
    let mut records = 0;
    let mut last = None;
    for reference in references.incoming(id.0) {
        let source = reference.source();
        if last != Some(source) {
            records += 1;
            last = Some(source);
        }
        if lines.len() >= MAX_INCOMING_LINES {
            continue;
        }
        let Some(record) = forge.record(source) else {
            continue;
        };
        lines.push(FieldLine {
            depth: 0,
            name: reference.location().into(),
            value: LineValue::Reference(RecordLink {
                guid: record.guid(),
                record_path: record.record_path().unwrap_or_default().into(),
                type_name: record.type_name().unwrap_or_default().into(),
                at: None,
                target: Some(RecordId(source)),
            }),
        });
    }
    lines.shrink_to_fit();
    IncomingView {
        lines,
        references: total,
        records,
        incomplete: references.is_truncated() || references.unreadable_records() > 0,
    }
}
