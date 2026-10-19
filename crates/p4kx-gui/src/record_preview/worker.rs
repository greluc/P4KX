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
            .spawn(move || {
                let source = match index.forge().read(&index, || {}) {
                    Ok(source) => source,
                    Err(failure) => {
                        let unread = || match &failure {
                            ReadFailure::ArchiveChanged(_) => Unread::ArchiveChanged,
                            ReadFailure::Unreadable(reason) => Unread::Failed(reason.clone()),
                        };
                        return serve(&waiting, &answer, &*wake, |_| Err(unread()));
                    }
                };
                match source.entry.open() {
                    Ok(forge) => serve(&waiting, &answer, &*wake, |id| read(&index, &forge, id)),
                    Err(error) => {
                        let reason = describe(&error);
                        serve(&waiting, &answer, &*wake, |_| {
                            Err(Unread::Failed(reason.clone()))
                        });
                    }
                }
            })
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

fn serve(
    waiting: &Receiver<(u64, RecordId)>,
    answer: &Sender<Answer>,
    wake: &dyn Fn(),
    mut read: impl FnMut(RecordId) -> Result<RecordView, Unread>,
) {
    while let Ok(mut request) = waiting.recv() {
        while let Ok(newer) = waiting.try_recv() {
            request = newer;
        }
        let (generation, id) = request;
        if answer.send((generation, read(id))).is_err() {
            return;
        }
        wake();
    }
}
