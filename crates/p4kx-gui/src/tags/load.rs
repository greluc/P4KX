use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::TagTree;

use crate::EntryIndex;
use crate::forge_cache::ReadFailure;
use crate::model::describe;

/// How reading the tag tree ended.
#[derive(Debug)]
pub(super) enum Outcome {
    Ready(Arc<TagTree>),
    Failed(Option<String>),
    ArchiveChanged,
}

/// The tree being read on a background thread; dropping it abandons the result.
#[derive(Debug)]
pub(super) struct Load {
    result: Receiver<Outcome>,
}

impl Load {
    pub(super) fn poll(&self) -> Option<Outcome> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Outcome::Failed(None)),
        }
    }
}

/// Starts reading the tags of `index`, calling `wake` from the thread once it ended.
pub(super) fn start(
    index: &Arc<EntryIndex>,
    wake: impl Fn() + Send + 'static,
) -> Result<Load, String> {
    let (sender, result) = mpsc::channel();
    let index = Arc::clone(index);
    thread::Builder::new()
        .name("tags".to_owned())
        .spawn(move || {
            if sender.send(read(&index)).is_ok() {
                wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Load { result })
}

fn read(index: &Arc<EntryIndex>) -> Outcome {
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    let tags = match index.forge().read(index, || {}) {
        Ok(source) => source.tags(),
        Err(failure) => Err(failure),
    };
    match tags {
        Ok(_) if index.watch().check().is_err() => Outcome::ArchiveChanged,
        Ok(tags) => Outcome::Ready(tags),
        Err(ReadFailure::ArchiveChanged(_)) => Outcome::ArchiveChanged,
        Err(ReadFailure::Unreadable(reason)) => Outcome::Failed(Some(reason)),
    }
}
