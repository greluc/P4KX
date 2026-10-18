use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::thread;

use p4kx_comparison::{
    Archive, CancelToken, CompareOptions, ComparisonError, ComparisonProgress, DataForgeEntry,
    Levels, OwnedComparison, OwnedDataForges, RecompareError,
};

use super::record_list::RecordChangeList;
use super::{ComparisonStage, ComparisonView};
use crate::model::describe;

/// How a Comparison ended.
#[derive(Debug)]
pub(crate) enum Outcome {
    Finished(Box<ComparisonView>),
    Failed(Option<String>),
    Cancelled,
}

/// A Comparison running on a background thread.
#[derive(Debug)]
pub(crate) struct Run {
    cancel: CancelToken,
    alive: Weak<()>,
    stage: Arc<Mutex<ComparisonStage>>,
    result: Receiver<Outcome>,
}

impl Run {
    /// Returns how far the Comparison has come.
    pub(crate) fn stage(&self) -> ComparisonStage {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Asks the thread to stop at its next check; returns a handle that lives as long as the
    /// thread.
    pub(crate) fn cancel(&self) -> Weak<()> {
        self.cancel.cancel();
        Weak::clone(&self.alive)
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

/// Starts comparing the Archives at `old` and `new` on a background thread, calling `wake` from
/// it as it advances and once it ended.
pub(crate) fn start(
    (old, new): (PathBuf, PathBuf),
    levels: Levels,
    wake: impl Fn() + Send + Sync + 'static,
) -> Result<Run, String> {
    let cancel = CancelToken::new();
    let stage = Arc::new(Mutex::new(ComparisonStage::Opening));
    let (sender, result) = mpsc::channel();
    let thread_cancel = cancel.clone();
    let thread_stage = Arc::clone(&stage);
    let alive = Arc::new(());
    let thread_alive = Arc::clone(&alive);
    thread::Builder::new()
        .name("comparison".to_owned())
        .spawn(move || {
            let outcome = run((&old, &new), levels, &thread_cancel, &thread_stage, &wake);
            if sender.send(outcome).is_ok() {
                wake();
            }
            drop(thread_alive);
        })
        .map_err(|error| describe(&error))?;
    Ok(Run {
        cancel,
        alive: Arc::downgrade(&alive),
        stage,
        result,
    })
}

fn open(path: &Path) -> Result<Arc<Archive>, String> {
    Archive::open(path)
        .map(Arc::new)
        .map_err(|error| format!("{}: {}", path.display(), describe(&error)))
}

fn open_both(old: &Path, new: &Path) -> Result<(Arc<Archive>, Arc<Archive>), String> {
    let (old, new) = rayon::join(|| open(old), || open(new));
    Ok((old?, new?))
}

/// Reads the DataForge Entry of `archive` and checks that it opens.
fn read_forge(archive: &Archive) -> Result<Arc<DataForgeEntry>, String> {
    let fail = |reason: String| format!("{}: {reason}", archive.path().display());
    let entry = DataForgeEntry::read(archive).map_err(|error| fail(describe(&error)))?;
    entry.open().map_err(|error| fail(describe(&error)))?;
    Ok(Arc::new(entry))
}

fn read_forges(old: &Archive, new: &Archive) -> Result<OwnedDataForges, String> {
    let (old, new) = rayon::join(|| read_forge(old), || read_forge(new));
    Ok(OwnedDataForges {
        old: old?,
        new: new?,
    })
}

fn run(
    (old, new): (&Path, &Path),
    levels: Levels,
    cancel: &CancelToken,
    stage: &Mutex<ComparisonStage>,
    wake: &(impl Fn() + Sync),
) -> Outcome {
    let set = |now: ComparisonStage| {
        *stage.lock().unwrap_or_else(PoisonError::into_inner) = now;
        wake();
    };
    let (old, new) = match open_both(old, new) {
        Ok(both) => both,
        Err(reason) => return Outcome::Failed(Some(reason)),
    };
    let forges = match levels.records.then(|| read_forges(&old, &new)).transpose() {
        Ok(forges) => forges,
        Err(reason) => return Outcome::Failed(Some(reason)),
    };
    if cancel.is_cancelled() {
        return Outcome::Cancelled;
    }
    let on_progress = |progress: ComparisonProgress| {
        set(ComparisonStage::Comparing {
            done: progress.entries_done,
            total: progress.entries_total,
        });
    };
    let options = CompareOptions {
        levels,
        filter: None,
        cancel: Some(cancel),
        progress: Some(&on_progress),
    };
    let comparison = match OwnedComparison::compare(old, new, forges, &options) {
        Ok(comparison) => comparison,
        Err(ComparisonError::Cancelled) => return Outcome::Cancelled,
        Err(error) => return Outcome::Failed(Some(describe(&error))),
    };
    let records = match levels.records.then(|| records(&comparison, cancel, &set)) {
        None => None,
        Some(Ok(list)) => Some(list),
        Some(Err(outcome)) => return outcome,
    };
    if cancel.is_cancelled() {
        return Outcome::Cancelled;
    }
    set(ComparisonStage::Preparing);
    ComparisonView::new(Arc::new(comparison), records, cancel)
        .map_or(Outcome::Cancelled, |view| Outcome::Finished(Box::new(view)))
}

/// Compares the Records of `comparison` field by field and lists the changes.
fn records(
    comparison: &OwnedComparison,
    cancel: &CancelToken,
    set: &(impl Fn(ComparisonStage) + Sync),
) -> Result<RecordChangeList, Outcome> {
    let on_progress = |progress: ComparisonProgress| {
        set(ComparisonStage::ComparingRecords {
            done: progress.records_done,
            total: progress.records_total,
        });
    };
    let listed = comparison.record_changes(None, Some(cancel), Some(&on_progress), |changes| {
        RecordChangeList::build(changes, cancel)
    });
    match listed {
        Ok(Some(list)) => Ok(list),
        Ok(None) | Err(RecompareError::Comparison(ComparisonError::Cancelled)) => {
            Err(Outcome::Cancelled)
        }
        Err(error) => Err(Outcome::Failed(Some(describe(&error)))),
    }
}
