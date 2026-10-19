use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use p4kx_extraction::DataForge;

use super::{RecordView, lines};
use crate::forge_cache::ReadFailure;
use crate::model::describe;
use crate::{EntryIndex, RecordId};

/// Why a Record could not be read for the preview.
#[derive(Debug)]
pub(super) enum Unread {
    Failed(String),
    ArchiveChanged,
}

pub(super) type Answer = (u64, Result<RecordView, Unread>);

/// A thread that keeps the DataForge of one Archive open and reads the Records the preview asks
/// for, only the newest of the requests waiting; it ends once this handle is dropped.
#[derive(Debug)]
pub(super) struct Worker {
    requests: Sender<(u64, RecordId)>,
    answers: Receiver<Answer>,
}

/// What the worker answered since the last poll.
pub(super) enum Answered {
    Nothing,
    Answer(Answer),
    Gone,
}

impl Worker {
    /// Starts the worker over the DataForge of `index`, read through the cache of the Archive,
    /// calling `wake` from it after each answer.
    pub(super) fn start(
        index: &Arc<EntryIndex>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let (requests, waiting) = mpsc::channel();
        let (answer, answers) = mpsc::channel();
        let index = Arc::clone(index);
        thread::Builder::new()
            .name("record-preview".to_owned())
            .spawn(move || run(&index, &waiting, &answer, &*wake))
            .map_err(|error| describe(&error))?;
        Ok(Self { requests, answers })
    }

    /// Asks for the Record `id` as request `generation`; returns whether the worker still runs.
    pub(super) fn request(&self, generation: u64, id: RecordId) -> bool {
        self.requests.send((generation, id)).is_ok()
    }

    /// Takes the next answer.
    pub(super) fn poll(&self) -> Answered {
        match self.answers.try_recv() {
            Ok(answer) => Answered::Answer(answer),
            Err(TryRecvError::Empty) => Answered::Nothing,
            Err(TryRecvError::Disconnected) => Answered::Gone,
        }
    }
}

fn run(
    index: &EntryIndex,
    waiting: &Receiver<(u64, RecordId)>,
    answer: &Sender<Answer>,
    wake: &dyn Fn(),
) {
    let mut first = None;
    loop {
        let reason = match index.forge().read(index, || {}) {
            Ok(source) => match source.entry.open() {
                Ok(forge) => {
                    return serve(waiting, answer, wake, first, |id| read(index, &forge, id));
                }
                Err(error) => Unread::Failed(describe(&error)),
            },
            Err(ReadFailure::ArchiveChanged(_)) => Unread::ArchiveChanged,
            Err(ReadFailure::Unreadable(reason)) => Unread::Failed(reason),
        };
        let Some(request) = first.take().or_else(|| next(waiting)) else {
            return;
        };
        if answer.send((request.0, Err(reason))).is_err() {
            return;
        }
        wake();
        let Some(retry) = next(waiting) else {
            return;
        };
        first = Some(retry);
    }
}

fn read(index: &EntryIndex, forge: &DataForge<'_>, id: RecordId) -> Result<RecordView, Unread> {
    let record = forge
        .record(id.0)
        .ok_or_else(|| Unread::Failed(format!("the DataForge has no Record {}", id.0)))?;
    let tree = record
        .tree()
        .map_err(|error| Unread::Failed(describe(&error)))?;
    let view = lines::view(forge, &tree);
    if index.watch().check().is_err() {
        return Err(Unread::ArchiveChanged);
    }
    Ok(view)
}

fn next(waiting: &Receiver<(u64, RecordId)>) -> Option<(u64, RecordId)> {
    let mut request = waiting.recv().ok()?;
    while let Ok(newer) = waiting.try_recv() {
        request = newer;
    }
    Some(request)
}

fn serve(
    waiting: &Receiver<(u64, RecordId)>,
    answer: &Sender<Answer>,
    wake: &dyn Fn(),
    mut first: Option<(u64, RecordId)>,
    mut read: impl FnMut(RecordId) -> Result<RecordView, Unread>,
) {
    while let Some(request) = first.take().or_else(|| next(waiting)) {
        let (generation, id) = request;
        if answer.send((generation, read(id))).is_err() {
            return;
        }
        wake();
    }
}
