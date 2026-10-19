use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use p4kx_extraction::query::{ColumnKind, Query, QueryRun};
use p4kx_extraction::{CancelToken, DataForgeEntry, QUERY_PAGE_SIZE};
use rayon::prelude::*;

use super::forge_slot::{ForgeSlot, ReadFailure};
use super::result::{QueryResult, ResultColumn, ResultRow};
use super::{QueryInvalid, QueryStage};
use crate::EntryIndex;
use crate::model::describe;

/// How a Query run ended.
#[derive(Debug)]
pub(crate) enum Outcome {
    Finished(Box<QueryResult>),
    Invalid(QueryInvalid),
    Failed(Option<String>),
    ArchiveChanged,
    Cancelled,
}

/// A Query running on a background thread.
#[derive(Debug)]
pub(crate) struct Run {
    cancel: CancelToken,
    stage: Arc<Mutex<QueryStage>>,
    result: Receiver<Outcome>,
}

impl Drop for Run {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl Run {
    /// Returns how far the run has come.
    pub(crate) fn stage(&self) -> QueryStage {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes the outcome once the thread ended.
    pub(crate) fn poll(&self) -> Option<Outcome> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Outcome::Failed(None)),
        }
    }
}

/// What a run needs: the open Archive, the parsed Query and its text, and the slot of the
/// Archive's DataForge.
pub(crate) struct Job {
    pub(crate) index: Arc<EntryIndex>,
    pub(crate) query: Query,
    pub(crate) text: String,
    pub(crate) forge: Arc<ForgeSlot>,
}

/// Starts running `job` on a background thread, calling `wake` from it as it advances and once
/// it ended.
pub(crate) fn start(job: Job, wake: impl Fn() + Send + Sync + 'static) -> Result<Run, String> {
    let cancel = CancelToken::new();
    let first = if job.forge.is_read() {
        QueryStage::Selecting
    } else {
        QueryStage::Loading
    };
    let stage = Arc::new(Mutex::new(first));
    let (sender, result) = mpsc::channel();
    let thread_cancel = cancel.clone();
    let thread_stage = Arc::clone(&stage);
    thread::Builder::new()
        .name("query".to_owned())
        .spawn(move || {
            let set = |now: QueryStage| {
                *thread_stage.lock().unwrap_or_else(PoisonError::into_inner) = now;
                wake();
            };
            let ended = run(job, &thread_cancel, &set);
            if sender.send(ended).is_ok() {
                wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Run {
        cancel,
        stage,
        result,
    })
}

fn run(job: Job, cancel: &CancelToken, set: &impl Fn(QueryStage)) -> Outcome {
    if job.index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    let entry = match job.forge.read(&job.index, || set(QueryStage::Loading)) {
        Ok(entry) => entry,
        Err(ReadFailure::ArchiveChanged) => return Outcome::ArchiveChanged,
        Err(ReadFailure::Unreadable(reason)) => return Outcome::Failed(Some(reason)),
    };
    let source = Source {
        entry: &entry,
        archive: job.index.archive().path(),
    };
    let outcome = query_over(source, &job.query, job.text, cancel, set);
    if matches!(outcome, Outcome::Finished(_)) && job.index.watch().check().is_err() {
        job.forge.clear();
        return Outcome::ArchiveChanged;
    }
    outcome
}

/// The DataForge a Query runs over and the Archive it was read from.
#[derive(Clone, Copy)]
struct Source<'a> {
    entry: &'a Arc<DataForgeEntry>,
    archive: &'a Path,
}
fn query_over(
    source: Source<'_>,
    query: &Query,
    text: String,
    cancel: &CancelToken,
    set: &impl Fn(QueryStage),
) -> Outcome {
    if cancel.is_cancelled() {
        return Outcome::Cancelled;
    }
    let forge = match source.entry.open() {
        Ok(forge) => forge,
        Err(error) => return Outcome::Failed(Some(describe(&error))),
    };
    set(QueryStage::Selecting);
    let compiled = match query.compile(&forge) {
        Ok(compiled) => compiled,
        Err(error) => return Outcome::Invalid(QueryInvalid::new(text, &error)),
    };
    let lists: Vec<bool> = compiled
        .column_kinds()
        .iter()
        .map(ColumnKind::is_multi_valued)
        .collect();
    let Some(read) = read_rows(&compiled.run(), &lists, cancel, set) else {
        return Outcome::Cancelled;
    };
    let columns = compiled
        .columns()
        .iter()
        .zip(compiled.column_kinds())
        .map(|(heading, kind)| ResultColumn::new(heading, kind))
        .collect();
    Outcome::Finished(Box::new(QueryResult::new(
        (text, columns),
        read.rows,
        (read.failed, read.unresolved),
        (Arc::clone(source.entry), source.archive.to_path_buf()),
    )))
}

/// The rows a run built, the Records that gave none and the unresolved References in the rows.
struct Read {
    rows: Vec<ResultRow>,
    failed: u64,
    unresolved: u64,
}

/// Builds the rows of `run` page by page, `lists` telling which columns hold lists; `None` once
/// cancelled.
fn read_rows(
    run: &QueryRun<'_, '_>,
    lists: &[bool],
    cancel: &CancelToken,
    set: &impl Fn(QueryStage),
) -> Option<Read> {
    let total = run.len();
    let mut read = Read {
        rows: Vec::with_capacity(total),
        failed: 0,
        unresolved: 0,
    };
    set(QueryStage::Reading { done: 0, total });
    for page in run.pages(QUERY_PAGE_SIZE) {
        if cancel.is_cancelled() {
            return None;
        }
        let built: Vec<Option<(ResultRow, usize)>> = page
            .par_iter()
            .map_init(String::new, |buffer, row| {
                ResultRow::of(row, lists, buffer).map(|built| (built, row.unresolved_references()))
            })
            .collect();
        for row in built {
            match row {
                Some((row, links)) => {
                    read.rows.push(row);
                    read.unresolved = read
                        .unresolved
                        .saturating_add(u64::try_from(links).unwrap_or(u64::MAX));
                }
                None => read.failed = read.failed.saturating_add(1),
            }
        }
        let done = read
            .rows
            .len()
            .saturating_add(usize::try_from(read.failed).unwrap_or(usize::MAX));
        set(QueryStage::Reading { done, total });
    }
    (!cancel.is_cancelled()).then_some(read)
}
