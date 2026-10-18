use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::SystemTime;

use p4kx_export::{
    CancelToken, ConflictPolicy, RecordExportOptions, RecordForm, write_planned_record_export,
};
use p4kx_extraction::{DataForgeEntry, Filter, Record, create_default_target};
use rayon::prelude::*;

use super::RecordExportSummary;
use super::source::ForgeSource;
use crate::extraction::target_naming;
use crate::model::describe;
use crate::{EntryIndex, TargetKind};

/// The Records a Record export writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selection {
    /// The Records at these positions in definition order.
    Records(Arc<[u32]>),
    /// The Records whose record path matches this Filter text.
    Filter(String),
}

/// What a Record export writes and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) selection: Selection,
    pub(crate) form: RecordForm,
    pub(crate) kind: TargetKind,
    pub(crate) folder: PathBuf,
    pub(crate) policy: ConflictPolicy,
}

/// The step a running Record export is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordExportStage {
    /// DataForge is read from the Archive.
    Reading,
    /// The Records to write are chosen.
    Selecting,
    /// The Records are written.
    Writing,
}

/// How far a running Record export has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordExportProgress {
    /// The step it is at.
    pub stage: RecordExportStage,
    /// Records written, kept or failed so far.
    pub done: usize,
    /// Records to write, known once writing starts.
    pub total: usize,
}

/// Where a Record export stands.
#[derive(Debug, Clone, Copy)]
pub enum RecordExportStatus<'a> {
    /// The export runs; the progress so far.
    Running(RecordExportProgress),
    /// Every Record ended, or the export was cancelled.
    Finished(&'a RecordExportSummary),
    /// DataForge or the target could not be prepared, for the reason given, or the export
    /// stopped without a result.
    Failed(Option<&'a str>),
}

#[derive(Debug, Default)]
struct Shared {
    stage: AtomicU8,
    done: AtomicUsize,
    total: AtomicUsize,
}

impl Shared {
    fn enter(&self, stage: RecordExportStage) {
        self.stage.store(stage as u8, Ordering::Relaxed);
    }

    fn progress(&self) -> RecordExportProgress {
        let stage = match self.stage.load(Ordering::Relaxed) {
            0 => RecordExportStage::Reading,
            1 => RecordExportStage::Selecting,
            _ => RecordExportStage::Writing,
        };
        RecordExportProgress {
            stage,
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }
}

type RunResult = Result<RecordExportSummary, String>;

#[derive(Debug)]
enum State {
    Running(Receiver<RunResult>),
    Finished(RecordExportSummary),
    Failed(Option<String>),
}

/// A Record export running on a background thread, or its outcome.
#[derive(Debug)]
pub struct RecordExport {
    folder: PathBuf,
    cancel: CancelToken,
    shared: Arc<Shared>,
    state: State,
}

impl RecordExport {
    /// Returns the chosen folder: the parent of a new folder, or the existing folder.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Returns where the export stands.
    #[must_use]
    pub fn status(&self) -> RecordExportStatus<'_> {
        match &self.state {
            State::Running(_) => RecordExportStatus::Running(self.shared.progress()),
            State::Finished(summary) => RecordExportStatus::Finished(summary),
            State::Failed(reason) => RecordExportStatus::Failed(reason.as_deref()),
        }
    }

    /// Returns whether the export still runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Returns whether the export was asked to stop.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Asks the export to stop; Records not yet written are reported as cancelled.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Takes the outcome once the export ended; returns whether it ended.
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

    /// Cancels the export and waits until it ended, partial files removed.
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

/// Starts writing the Records of `request` from the DataForge of `index` on a background thread,
/// reading DataForge into `source` unless it holds it already, and calls `wake` from that thread
/// once the export ended.
pub(crate) fn start(
    index: &Arc<EntryIndex>,
    source: Arc<OnceLock<ForgeSource>>,
    request: Request,
    wake: impl Fn() + Send + 'static,
) -> RecordExport {
    let cancel = CancelToken::new();
    let shared = Arc::new(Shared::default());
    let folder = request.folder.clone();
    let (sender, result) = mpsc::channel();
    let thread_index = Arc::clone(index);
    let thread_cancel = cancel.clone();
    let thread_shared = Arc::clone(&shared);
    let spawned = thread::Builder::new()
        .name("record-export".to_owned())
        .spawn(move || {
            let job = Job {
                index: &thread_index,
                request: &request,
                cancel: &thread_cancel,
                shared: &thread_shared,
            };
            if sender.send(job.run(&source)).is_ok() {
                wake();
            }
        });
    let state = match spawned {
        Ok(_handle) => State::Running(result),
        Err(error) => State::Failed(Some(describe(&error))),
    };
    RecordExport {
        folder,
        cancel,
        shared,
        state,
    }
}

struct Job<'a> {
    index: &'a EntryIndex,
    request: &'a Request,
    cancel: &'a CancelToken,
    shared: &'a Shared,
}

impl Job<'_> {
    fn run(&self, slot: &OnceLock<ForgeSource>) -> RunResult {
        let source = self.source(slot)?;
        let forge = source.entry.open().map_err(|error| describe(&error))?;
        self.shared.enter(RecordExportStage::Selecting);
        let all: Vec<Record<'_>> = forge.records().collect();
        let records = select(&all, &self.request.selection);
        if records.is_empty() || self.cancel.is_cancelled() {
            return Ok(RecordExportSummary::nothing_written(
                self.request.form,
                self.request.folder.clone(),
                records.len(),
                self.cancel.is_cancelled(),
            ));
        }
        let target = prepare_target(self.index, self.request).map_err(|error| describe(&error))?;
        self.shared.total.store(records.len(), Ordering::Relaxed);
        self.shared.enter(RecordExportStage::Writing);
        let progress = || {
            self.shared.done.fetch_add(1, Ordering::Relaxed);
        };
        let options = RecordExportOptions {
            policy: self.request.policy,
            cancel: Some(self.cancel),
            progress: Some(&progress),
        };
        let summary = write_planned_record_export(
            &forge,
            &source.plans,
            &records,
            self.request.form,
            &target,
            &options,
        );
        Ok(RecordExportSummary::new(target, records.len(), &summary))
    }

    /// Returns DataForge from `slot`, reading it from the Archive first when the slot is empty.
    fn source<'s>(&self, slot: &'s OnceLock<ForgeSource>) -> Result<&'s ForgeSource, String> {
        self.index
            .watch()
            .check()
            .map_err(|error| describe(&error))?;
        if let Some(source) = slot.get() {
            return Ok(source);
        }
        let entry = DataForgeEntry::read(self.index.archive()).map_err(|error| describe(&error))?;
        self.index
            .watch()
            .check()
            .map_err(|error| describe(&error))?;
        Ok(slot.get_or_init(|| ForgeSource {
            entry,
            plans: p4kx_export::RecordFilePlans::default(),
        }))
    }
}

/// Returns the Records of `all` that `selection` names, each once.
fn select<'f>(all: &[Record<'f>], selection: &Selection) -> Vec<Record<'f>> {
    match selection {
        Selection::Records(positions) => {
            let mut positions = positions.to_vec();
            positions.sort_unstable();
            positions.dedup();
            positions
                .iter()
                .filter_map(|&position| all.get(position as usize).copied())
                .collect()
        }
        Selection::Filter(text) => {
            let filter = Filter::new(text);
            all.par_iter()
                .filter(|record| record.record_path().is_ok_and(|path| filter.matches(path)))
                .copied()
                .collect()
        }
    }
}

fn prepare_target(index: &EntryIndex, request: &Request) -> io::Result<PathBuf> {
    match request.kind {
        TargetKind::ExistingFolder => {
            fs::create_dir_all(&request.folder)?;
            Ok(request.folder.clone())
        }
        TargetKind::NewFolder => {
            let (channel, build) = target_naming(index.archive());
            create_default_target(
                &request.folder,
                channel.as_deref(),
                build,
                SystemTime::now(),
            )
        }
    }
}
