use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::SystemTime;

use p4kx_export::{CancelToken, CatalogError, CatalogOptions, CatalogProgress, build_catalog_from};
use p4kx_extraction::is_same_file;

use super::write::{Written, write_replacing};
use super::{CatalogExportProgress, CatalogExportStage, CatalogExportSummary, Shared};
use crate::EntryIndex;
use crate::forge_cache::ForgeSlot;
use crate::model::describe;

/// Where a Basetool catalog stands.
#[derive(Debug, Clone, Copy)]
pub enum CatalogExportStatus<'a> {
    /// The catalog is being created; the progress so far.
    Running(CatalogExportProgress),
    /// The catalog file was written.
    Finished(&'a CatalogExportSummary),
    /// The catalog was cancelled; no file was written.
    Cancelled,
    /// The chosen file is the open Archive; nothing was written.
    IsArchive,
    /// The catalog could not be created, for the reason given, or it stopped without a result.
    Failed(Option<&'a str>),
}

enum Outcome {
    Written(CatalogExportSummary),
    Cancelled,
    IsArchive,
}

type RunResult = Result<Outcome, String>;

#[derive(Debug)]
enum State {
    Running(Receiver<RunResult>),
    Finished(CatalogExportSummary),
    Cancelled,
    IsArchive,
    Failed(Option<String>),
}

/// A Basetool catalog created on a background thread, or its outcome.
#[derive(Debug)]
pub struct CatalogExport {
    id: u64,
    file: PathBuf,
    shown: String,
    cancel: CancelToken,
    shared: Arc<Shared>,
    state: State,
}

impl CatalogExport {
    fn new(file: PathBuf, cancel: CancelToken, shared: Arc<Shared>, state: State) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            shown: file.display().to_string(),
            file,
            cancel,
            shared,
            state,
        }
    }

    /// Returns the number that tells this catalog apart from every other one of the process.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the catalog file chosen.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the catalog file chosen as the windows show it.
    #[must_use]
    pub fn shown_file(&self) -> &str {
        &self.shown
    }

    /// Returns where the catalog stands.
    #[must_use]
    pub fn status(&self) -> CatalogExportStatus<'_> {
        match &self.state {
            State::Running(_) => CatalogExportStatus::Running(self.shared.progress()),
            State::Finished(summary) => CatalogExportStatus::Finished(summary),
            State::Cancelled => CatalogExportStatus::Cancelled,
            State::IsArchive => CatalogExportStatus::IsArchive,
            State::Failed(reason) => CatalogExportStatus::Failed(reason.as_deref()),
        }
    }

    /// Returns whether the catalog is still being created.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Returns whether the catalog was asked to stop.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Asks the catalog to stop; no file is written then.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Takes the outcome once the catalog ended; returns whether it ended.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Running(result) = &self.state else {
            return false;
        };
        self.state = match result.try_recv() {
            Ok(outcome) => ended(Some(outcome)),
            Err(TryRecvError::Empty) => return self.cancel.is_cancelled() && self.give_up(),
            Err(TryRecvError::Disconnected) => ended(None),
        };
        true
    }

    /// Cancels the catalog and waits until it ended, its temporary file removed.
    pub(crate) fn stop(&mut self) {
        self.cancel.cancel();
        if self.give_up() {
            return;
        }
        let State::Running(result) = &self.state else {
            return;
        };
        self.state = ended(result.recv().ok());
    }

    /// Ends a cancelled catalog at once while it still reads DataForge, which writes nothing;
    /// the thread finishes reading and keeps DataForge for the next export. Returns whether it
    /// ended.
    fn give_up(&mut self) -> bool {
        if !matches!(self.state, State::Running(_)) || !self.shared.gate.give_up() {
            return false;
        }
        self.state = State::Cancelled;
        true
    }
}

fn ended(outcome: Option<RunResult>) -> State {
    match outcome {
        Some(Ok(Outcome::Written(summary))) => State::Finished(summary),
        Some(Ok(Outcome::Cancelled)) => State::Cancelled,
        Some(Ok(Outcome::IsArchive)) => State::IsArchive,
        Some(Err(reason)) => State::Failed(Some(reason)),
        None => State::Failed(None),
    }
}

/// Creates the Basetool catalog of the Archive of `index` into `file` on a background thread,
/// reading DataForge into `slot` unless it holds it already, and calls `wake` from that thread
/// once the catalog ended.
pub(crate) fn start(
    index: &Arc<EntryIndex>,
    slot: Arc<ForgeSlot>,
    file: PathBuf,
    wake: impl Fn() + Send + 'static,
) -> CatalogExport {
    let cancel = CancelToken::new();
    let shared = Arc::new(Shared::default());
    let (sender, result) = mpsc::channel();
    let job = Job {
        index: Arc::clone(index),
        file: file.clone(),
        cancel: cancel.clone(),
        shared: Arc::clone(&shared),
    };
    let spawned = thread::Builder::new()
        .name("basetool-catalog".to_owned())
        .spawn(move || {
            if sender.send(job.run(&slot)).is_ok() {
                wake();
            }
        });
    let state = match spawned {
        Ok(_handle) => State::Running(result),
        Err(error) => State::Failed(Some(describe(&error))),
    };
    CatalogExport::new(file, cancel, shared, state)
}

/// Returns the outcome of a catalog refused because `file` is the open Archive.
pub(crate) fn refused(file: PathBuf) -> CatalogExport {
    CatalogExport::new(file, CancelToken::new(), Arc::default(), State::IsArchive)
}

struct Job {
    index: Arc<EntryIndex>,
    file: PathBuf,
    cancel: CancelToken,
    shared: Arc<Shared>,
}

impl Job {
    fn run(&self, slot: &ForgeSlot) -> RunResult {
        if is_same_file(&self.file, self.index.archive().path()) {
            return Ok(Outcome::IsArchive);
        }
        self.check_archive()?;
        let source = slot.read(&self.index)?;
        if !self.shared.gate.leave_reading() {
            return Ok(Outcome::Cancelled);
        }
        self.shared.enter(CatalogExportStage::Building);
        let progress = |progress: CatalogProgress| {
            self.shared
                .total
                .store(progress.records_total, Ordering::Relaxed);
            self.shared
                .done
                .fetch_max(progress.records_done, Ordering::Relaxed);
        };
        let options = CatalogOptions {
            filter: None,
            exported_at: SystemTime::now(),
            cancel: Some(&self.cancel),
            progress: Some(&progress),
        };
        let built = match build_catalog_from(self.index.archive(), &source.entry, &options) {
            Ok(built) => built,
            Err(CatalogError::Cancelled) => return Ok(Outcome::Cancelled),
            Err(error) => return Err(describe(&error)),
        };
        self.check_archive()?;
        self.shared.enter(CatalogExportStage::Writing);
        let (json, summary) = built.into_parts();
        let written = write_replacing(&self.file, &json, || self.cancel.is_cancelled())
            .map_err(|error| describe(&error))?;
        Ok(match written {
            Written::Done => Outcome::Written(CatalogExportSummary {
                file: self.file.clone(),
                summary,
            }),
            Written::Cancelled => Outcome::Cancelled,
        })
    }

    fn check_archive(&self) -> Result<(), String> {
        self.index.watch().check().map_err(|error| describe(&error))
    }
}
