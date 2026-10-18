use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use p4kx_extraction::query::{ColumnKind, Query};
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
    let run = compiled.run();
    let total = run.len();
    let mut rows = Vec::with_capacity(total);
    let mut failed = 0_u64;
    set(QueryStage::Reading { done: 0, total });
    for page in run.pages(QUERY_PAGE_SIZE) {
        if cancel.is_cancelled() {
            return Outcome::Cancelled;
        }
        let built: Vec<Option<ResultRow>> = page
            .par_iter()
            .map_init(String::new, |buffer, row| {
                ResultRow::of(row, &lists, buffer)
            })
            .collect();
        for row in built {
            match row {
                Some(row) => rows.push(row),
                None => failed = failed.saturating_add(1),
            }
        }
        let done = rows
            .len()
            .saturating_add(usize::try_from(failed).unwrap_or(usize::MAX));
        set(QueryStage::Reading { done, total });
    }
    if cancel.is_cancelled() {
        return Outcome::Cancelled;
    }
    let columns = compiled
        .columns()
        .iter()
        .zip(compiled.column_kinds())
        .map(|(heading, kind)| ResultColumn::new(heading, kind))
        .collect();
    Outcome::Finished(Box::new(QueryResult::new(
        (text, columns),
        rows,
        failed,
        (Arc::clone(source.entry), source.archive.to_path_buf()),
    )))
}
