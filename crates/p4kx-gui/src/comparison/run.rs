use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use p4kx_comparison::{
    Archive, CancelToken, CompareOptions, ComparisonError, ComparisonProgress, Levels,
    OwnedComparison,
};

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
    stage: Arc<Mutex<ComparisonStage>>,
    result: Receiver<Outcome>,
}

impl Run {
    /// Returns how far the Comparison has come.
    pub(crate) fn stage(&self) -> ComparisonStage {
        *self.stage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Asks the thread to stop at its next check.
    pub(crate) fn cancel(&self) {
        self.cancel.cancel();
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
    old: PathBuf,
    new: PathBuf,
    wake: impl Fn() + Send + Sync + 'static,
) -> Result<Run, String> {
    let cancel = CancelToken::new();
    let stage = Arc::new(Mutex::new(ComparisonStage::Opening));
    let (sender, result) = mpsc::channel();
    let thread_cancel = cancel.clone();
    let thread_stage = Arc::clone(&stage);
    thread::Builder::new()
        .name("comparison".to_owned())
        .spawn(move || {
            let outcome = run(&old, &new, &thread_cancel, &thread_stage, &wake);
            if sender.send(outcome).is_ok() {
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

fn open(path: &Path) -> Result<Arc<Archive>, String> {
    Archive::open(path)
        .map(Arc::new)
        .map_err(|error| format!("{}: {}", path.display(), describe(&error)))
}

fn run(
    old: &Path,
    new: &Path,
    cancel: &CancelToken,
    stage: &Mutex<ComparisonStage>,
    wake: &(impl Fn() + Sync),
) -> Outcome {
    let set = |now: ComparisonStage| {
        *stage.lock().unwrap_or_else(PoisonError::into_inner) = now;
        wake();
    };
    let (old, new) = rayon::join(|| open(old), || open(new));
    let (old, new) = match (old, new) {
        (Ok(old), Ok(new)) => (old, new),
        (Err(reason), _) | (_, Err(reason)) => return Outcome::Failed(Some(reason)),
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
        levels: Levels { entries: true },
        filter: None,
        cancel: Some(cancel),
        progress: Some(&on_progress),
    };
    let comparison = match OwnedComparison::compare(old, new, &options) {
        Ok(comparison) => comparison,
        Err(ComparisonError::Cancelled) => return Outcome::Cancelled,
        Err(error) => return Outcome::Failed(Some(describe(&error))),
    };
    if cancel.is_cancelled() {
        return Outcome::Cancelled;
    }
    set(ComparisonStage::Preparing);
    Outcome::Finished(Box::new(ComparisonView::new(Arc::new(comparison))))
}
