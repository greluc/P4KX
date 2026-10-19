use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::SystemTime;

use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, ArchiveWatch, Build, CancelToken, ConflictPolicy, Entry, ExtractOptions, Progress,
    create_default_target, extract,
};

use p4kx_settings::{ExtractionAudio, ExtractionTextures};

use super::{ExtractionSummary, TargetKind, audio_conversion, texture_conversion};
use crate::EntryIndex;
use crate::comparison::ChangeSelection;
use crate::model::describe;

/// The Entries an Extraction writes and the Archive they are read from.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    /// The Entries with the ids `ids` of the open Archive `index`.
    Index {
        /// The open Archive.
        index: Arc<EntryIndex>,
        /// The ids of the Entries.
        ids: Vec<u32>,
    },
    /// The added and changed Entries of one side of a Comparison.
    Changes(ChangeSelection),
}

impl Source {
    /// Returns the Archive the Entries are read from.
    pub(crate) fn archive(&self) -> &Archive {
        match self {
            Self::Index { index, .. } => index.archive(),
            Self::Changes(selection) => selection.archive(),
        }
    }

    /// Returns the number of Entries.
    pub(crate) const fn len(&self) -> usize {
        match self {
            Self::Index { ids, .. } => ids.len(),
            Self::Changes(selection) => selection.len(),
        }
    }
}

/// What an Extraction writes and where.
#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub(crate) source: Source,
    pub(crate) kind: TargetKind,
    pub(crate) folder: PathBuf,
    pub(crate) policy: ConflictPolicy,
    pub(crate) convert: bool,
    pub(crate) textures: ExtractionTextures,
    pub(crate) audio: ExtractionAudio,
}

type RunResult = Result<ExtractionSummary, String>;

#[derive(Debug)]
enum State {
    Running(Receiver<RunResult>),
    Finished(ExtractionSummary),
    Failed(Option<String>),
}

/// Where an Extraction stands.
#[derive(Debug, Clone, Copy)]
pub enum ExtractionStatus<'a> {
    /// The Entries are being written; the progress so far.
    Running(Progress),
    /// Every Entry ended, or the Extraction was cancelled.
    Finished(&'a ExtractionSummary),
    /// The target could not be prepared, for the reason given, or the Extraction stopped
    /// without a result.
    Failed(Option<&'a str>),
}

/// An Extraction running on a background thread, or its outcome.
#[derive(Debug)]
pub struct Extraction {
    archive: PathBuf,
    comparison_side: bool,
    folder: PathBuf,
    cancel: CancelToken,
    progress: Arc<Mutex<Progress>>,
    state: State,
}

impl Extraction {
    /// Returns the path of the Archive the Entries are read from.
    #[must_use]
    pub fn archive(&self) -> &Path {
        &self.archive
    }

    /// Returns whether the Entries are the changes of one side of a Comparison.
    #[must_use]
    pub const fn is_comparison_side(&self) -> bool {
        self.comparison_side
    }

    /// Returns the chosen folder: the parent of a new folder, or the existing folder.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Returns where the Extraction stands.
    #[must_use]
    pub fn status(&self) -> ExtractionStatus<'_> {
        match &self.state {
            State::Running(_) => ExtractionStatus::Running(
                *self.progress.lock().unwrap_or_else(PoisonError::into_inner),
            ),
            State::Finished(summary) => ExtractionStatus::Finished(summary),
            State::Failed(reason) => ExtractionStatus::Failed(reason.as_deref()),
        }
    }

    /// Returns whether the Extraction still runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Returns whether the Extraction was asked to stop.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Asks the Extraction to stop; Entries not yet complete are reported as cancelled.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Takes the outcome once the Extraction ended; returns whether it ended.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Running(result) = &self.state else {
            return false;
        };
        self.state = match result.try_recv() {
            Ok(outcome) => ended(Some(outcome)),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => ended(None),
        };
        true
    }

    /// Cancels the Extraction and waits until it ended, partial files removed.
    pub(crate) fn stop(&mut self) {
        let State::Running(result) = &self.state else {
            return;
        };
        self.cancel.cancel();
        self.state = ended(result.recv().ok());
    }
}

fn ended(outcome: Option<RunResult>) -> State {
    match outcome {
        Some(Ok(summary)) => State::Finished(summary),
        Some(Err(reason)) => State::Failed(Some(reason)),
        None => State::Failed(None),
    }
}

/// Returns the Channel name and the Build version a new target folder is named after.
pub(crate) fn target_naming(archive: &Archive) -> (Option<String>, Option<&str>) {
    (
        channel_name(archive.path()),
        archive.build().map(Build::version),
    )
}

/// Starts writing the Entries of `request` on a background thread, calling `wake` from it once
/// the Extraction ended.
pub(crate) fn start(request: Request, wake: impl Fn() + Send + 'static) -> Extraction {
    let cancel = CancelToken::new();
    let progress = Arc::new(Mutex::new(Progress {
        entries_total: request.source.len(),
        ..Progress::default()
    }));
    let folder = request.folder.clone();
    let archive = request.source.archive().path().to_path_buf();
    let comparison_side = matches!(request.source, Source::Changes(_));
    let (sender, result) = mpsc::channel();
    let thread_cancel = cancel.clone();
    let thread_progress = Arc::clone(&progress);
    let spawned = thread::Builder::new()
        .name("extraction".to_owned())
        .spawn(move || {
            let outcome = run(&request, &thread_cancel, &thread_progress);
            if sender.send(outcome).is_ok() {
                wake();
            }
        });
    let state = match spawned {
        Ok(_handle) => State::Running(result),
        Err(error) => State::Failed(Some(describe(&error))),
    };
    Extraction {
        archive,
        comparison_side,
        folder,
        cancel,
        progress,
        state,
    }
}

fn run(request: &Request, cancel: &CancelToken, progress: &Mutex<Progress>) -> RunResult {
    match &request.source {
        Source::Index { index, ids } => {
            let mut ids = ids.clone();
            ids.sort_unstable();
            let entries: Vec<_> = ids.iter().filter_map(|&id| index.entry(id)).collect();
            write(
                (index.archive(), &entries, index.watch()),
                request,
                cancel,
                progress,
            )
        }
        Source::Changes(selection) => {
            let archive = selection.archive();
            let watch = ArchiveWatch::new(archive);
            let entries = selection.entries();
            write((archive, &entries, &watch), request, cancel, progress)
        }
    }
}

fn write(
    (archive, entries, watch): (&Archive, &[&Entry], &ArchiveWatch),
    request: &Request,
    cancel: &CancelToken,
    progress: &Mutex<Progress>,
) -> RunResult {
    let target = prepare_target(archive, request).map_err(|error| describe(&error))?;
    let on_progress = |now: Progress| {
        *progress.lock().unwrap_or_else(PoisonError::into_inner) = now;
    };
    let options = ExtractOptions {
        policy: request.policy,
        convert: request.convert,
        textures: texture_conversion(request.textures),
        audio: audio_conversion(request.audio),
        cancel: Some(cancel),
        progress: Some(&on_progress),
        watch: Some(watch),
    };
    let report = extract(archive, entries, &target, &options);
    Ok(ExtractionSummary::new(target, &report))
}

fn prepare_target(archive: &Archive, request: &Request) -> io::Result<PathBuf> {
    match request.kind {
        TargetKind::ExistingFolder => {
            fs::create_dir_all(&request.folder)?;
            Ok(request.folder.clone())
        }
        TargetKind::NewFolder => {
            let (channel, build) = target_naming(archive);
            create_default_target(
                &request.folder,
                channel.as_deref(),
                build,
                SystemTime::now(),
            )
        }
    }
}
