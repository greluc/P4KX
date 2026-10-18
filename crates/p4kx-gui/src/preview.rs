mod contents;

use std::io::Read;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::Entry;

pub use contents::PreviewContents;
pub(crate) use contents::{HEX_ROW_LEN, printable};

use crate::EntryIndex;
use crate::model::describe;

/// The largest Entry, in bytes, the window previews; larger Entries are not read.
pub const PREVIEW_SIZE_LIMIT: u64 = 256 * 1024 * 1024;

/// The most bytes a preview reads from the start of an Entry; the rest is left out.
pub const PREVIEW_READ_LIMIT: usize = 1024 * 1024;

/// The tab a preview is shown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PreviewTab {
    /// The data decoded as text, line by line.
    #[default]
    Text,
    /// The data as offsets, hexadecimal bytes and printable characters.
    Hex,
    /// The summary of the Archive's DataForge instead of the selected Entry.
    DataForge,
}

/// Where the preview of an Entry stands.
#[derive(Debug, Clone, Copy)]
pub enum PreviewStatus<'a> {
    /// The Entry is being read.
    Loading,
    /// The Entry is larger than [`PREVIEW_SIZE_LIMIT`] and was not read.
    TooLarge,
    /// The Entry could not be read, for the reason given, or reading stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the Entry was not read or its data was dropped.
    ArchiveChanged,
    /// The Entry was read.
    Ready(&'a PreviewContents),
}

#[derive(Debug)]
enum Unread {
    Failed(Option<String>),
    ArchiveChanged,
}

type ReadResult = Result<PreviewContents, Unread>;

#[derive(Debug)]
enum State {
    Loading(Receiver<ReadResult>),
    TooLarge,
    Failed(Option<String>),
    ArchiveChanged,
    Ready(PreviewContents),
}

/// The preview of one Entry, read on a background thread.
#[derive(Debug)]
pub struct Preview {
    entry: u32,
    state: State,
}

impl Preview {
    /// Starts reading the Entry `id` of `index` on a background thread, calling `wake` from it
    /// once the data is ready; reads on the calling thread when no thread can be started.
    ///
    /// Nothing is read from an Archive that changed on disk, and data read while it changed is
    /// dropped.
    pub(crate) fn start(
        index: &Arc<EntryIndex>,
        id: u32,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        if index.watch().check().is_err() {
            return Self {
                entry: id,
                state: State::ArchiveChanged,
            };
        }
        let state = match index.entry(id) {
            None => State::Failed(None),
            Some(entry) if entry.uncompressed_size() > PREVIEW_SIZE_LIMIT => State::TooLarge,
            Some(_) => spawn(index, id, wake),
        };
        Self { entry: id, state }
    }

    /// Returns the id of the previewed Entry.
    #[must_use]
    pub const fn entry(&self) -> u32 {
        self.entry
    }

    /// Returns where the preview stands.
    #[must_use]
    pub fn status(&self) -> PreviewStatus<'_> {
        match &self.state {
            State::Loading(_) => PreviewStatus::Loading,
            State::TooLarge => PreviewStatus::TooLarge,
            State::Failed(reason) => PreviewStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => PreviewStatus::ArchiveChanged,
            State::Ready(contents) => PreviewStatus::Ready(contents),
        }
    }

    /// Returns whether the Entry is still being read.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self.state, State::Loading(_))
    }

    /// Returns the data read, or `None` while reading or when nothing was read.
    #[must_use]
    pub const fn contents(&self) -> Option<&PreviewContents> {
        match &self.state {
            State::Ready(contents) => Some(contents),
            _ => None,
        }
    }

    /// Takes the data once it was read; returns whether reading finished.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Loading(result) = &self.state else {
            return false;
        };
        self.state = match result.try_recv() {
            Ok(read) => settled(read),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => State::Failed(None),
        };
        true
    }
}

fn settled(read: ReadResult) -> State {
    match read {
        Ok(contents) => State::Ready(contents),
        Err(Unread::Failed(reason)) => State::Failed(reason),
        Err(Unread::ArchiveChanged) => State::ArchiveChanged,
    }
}

fn spawn(index: &Arc<EntryIndex>, id: u32, wake: impl Fn() + Send + 'static) -> State {
    let (sender, result) = mpsc::channel();
    let thread_index = Arc::clone(index);
    let spawned = thread::Builder::new()
        .name("preview".to_owned())
        .spawn(move || {
            if sender.send(read(&thread_index, id)).is_ok() {
                wake();
            }
        });
    match spawned {
        Ok(_handle) => State::Loading(result),
        Err(_ignored) => settled(read(index, id)),
    }
}

fn read(index: &EntryIndex, id: u32) -> ReadResult {
    let entry = index.entry(id).ok_or(Unread::Failed(None))?;
    let bytes = read_start(index, entry);
    if index.watch().check().is_err() {
        return Err(Unread::ArchiveChanged);
    }
    let bytes = bytes.map_err(|reason| Unread::Failed(Some(reason)))?;
    Ok(PreviewContents::new(bytes, entry.uncompressed_size()))
}

fn read_start(index: &EntryIndex, entry: &Entry) -> Result<Vec<u8>, String> {
    let reader = index
        .archive()
        .open_entry(entry)
        .map_err(|error| describe(&error))?;
    let limit = u64::try_from(PREVIEW_READ_LIMIT).unwrap_or(u64::MAX);
    let capacity = usize::try_from(entry.uncompressed_size().min(limit)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| describe(&error))?;
    Ok(bytes)
}
