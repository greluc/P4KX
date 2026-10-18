use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use p4kx_extraction::query::Query;
use p4kx_extraction::{CancelToken, DataForgeEntry, QUERY_PAGE_SIZE};
use rayon::prelude::*;

use super::result::{QueryResult, ResultRow};
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

/// What the run thread hands back: how it ended and the DataForge it read, for the next run.
type Ended = (Outcome, Option<Arc<DataForgeEntry>>);

/// A Query running on a background thread.
#[derive(Debug)]
pub(crate) struct Run {
    cancel: CancelToken,
    stage: Arc<Mutex<QueryStage>>,
    result: Receiver<Ended>,
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
    pub(crate) fn poll(&self) -> Option<Ended> {
        match self.result.try_recv() {
            Ok(ended) => Some(ended),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some((Outcome::Failed(None), None)),
        }
    }
}

/// What a run needs: the open Archive, the parsed Query and its text, and the DataForge read
/// by an earlier run of the same Archive.
pub(crate) struct Job {
    pub(crate) index: Arc<EntryIndex>,
    pub(crate) query: Query,
    pub(crate) text: String,
    pub(crate) forge: Option<Arc<DataForgeEntry>>,
}

/// Starts running `job` on a background thread, calling `wake` from it as it advances and once
/// it ended.
pub(crate) fn start(job: Job, wake: impl Fn() + Send + Sync + 'static) -> Result<Run, String> {
    let cancel = CancelToken::new();
    let stage = Arc::new(Mutex::new(QueryStage::Selecting));
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

fn run(job: Job, cancel: &CancelToken, set: &impl Fn(QueryStage)) -> Ended {
    if job.index.watch().check().is_err() {
        return (Outcome::ArchiveChanged, None);
    }
    let entry = if let Some(entry) = job.forge {
        entry
    } else {
        set(QueryStage::Loading);
        match DataForgeEntry::read(job.index.archive()) {
            Ok(entry) => Arc::new(entry),
            Err(error) => return (Outcome::Failed(Some(describe(&error))), None),
        }
    };
    let outcome = query_over(&entry, &job.query, job.text, cancel, set);
    (outcome, Some(entry))
}

fn query_over(
    entry: &Arc<DataForgeEntry>,
    query: &Query,
    text: String,
    cancel: &CancelToken,
    set: &impl Fn(QueryStage),
) -> Outcome {
    if cancel.is_cancelled() {
        return Outcome::Cancelled;
    }
    let forge = match entry.open() {
        Ok(forge) => forge,
        Err(error) => return Outcome::Failed(Some(describe(&error))),
    };
    set(QueryStage::Selecting);
    let compiled = match query.compile(&forge) {
        Ok(compiled) => compiled,
        Err(error) => return Outcome::Invalid(QueryInvalid::new(text, &error)),
    };
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
            .map_init(String::new, |buffer, row| ResultRow::of(row, buffer))
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
    let columns = compiled.columns().to_vec();
    Outcome::Finished(Box::new(QueryResult::new(
        text,
        columns,
        rows,
        failed,
        Arc::clone(entry),
    )))
}
