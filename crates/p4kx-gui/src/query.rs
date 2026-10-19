use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Weak};

use p4kx_extraction::query::{Query, QueryError, QuerySchema};
use p4kx_settings::SavedQueries;

mod cell;
mod completion;
mod export;
mod forge_slot;
mod result;
mod run;
mod saved;

pub use cell::CellLook;
pub(crate) use completion::{CompletionState, Trigger};
pub use export::{QueryExportFormat, QueryExportStatus};
pub use result::{QUERY_PAGE_ROWS, QueryResult, ResultColumn};
pub use saved::{NameNote, SavedDialog};

use crate::EntryIndex;
use export::ExportJob;
use forge_slot::ForgeSlot;
use run::{Job, Outcome, Run};

/// How far a running Query has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryStage {
    /// DataForge is read from the Archive.
    Loading,
    /// The Query is checked and the Records are selected.
    Selecting,
    /// The rows are read; `done` of `total` Records.
    Reading {
        /// Records read so far.
        done: usize,
        /// Records selected.
        total: usize,
    },
}

/// A Query that cannot be parsed or compiled, with the place in its text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryInvalid {
    query: String,
    message: String,
    span: Range<usize>,
    line: usize,
    column: usize,
}

impl QueryInvalid {
    pub(crate) fn new(query: String, error: &QueryError) -> Self {
        let floor = |offset: usize| {
            let mut offset = offset.min(query.len());
            while !query.is_char_boundary(offset) {
                offset -= 1;
            }
            offset
        };
        let start = floor(error.span().start());
        let end = floor(error.span().end()).max(start);
        let before = query.get(..start).unwrap_or_default();
        let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
        let line = before.matches('\n').count() + 1;
        let column = before.get(line_start..).unwrap_or_default().chars().count() + 1;
        Self {
            message: error.kind().to_string(),
            query,
            span: start..end,
            line,
            column,
        }
    }

    /// Returns the Query text the error was found in.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns what is wrong.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the bytes of the Query text the error marks; empty at the end of the text.
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// Returns the line of the error, counted from 1.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// Returns the column of the error in characters, counted from 1.
    #[must_use]
    pub const fn column(&self) -> usize {
        self.column
    }
}

/// Where the Query of the tab stands.
#[derive(Debug, Clone, Copy)]
pub enum QueryStatus<'a> {
    /// No Query ran yet.
    Idle,
    /// It runs off the UI thread.
    Running(QueryStage),
    /// It cannot be parsed or compiled.
    Invalid(&'a QueryInvalid),
    /// It was cancelled.
    Cancelled,
    /// It failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so DataForge was not read.
    ArchiveChanged,
    /// It finished.
    Finished(&'a QueryResult),
}

impl QueryStatus<'_> {
    /// Returns how far a running Query has come, or `None` when none runs.
    #[must_use]
    pub const fn stage(self) -> Option<QueryStage> {
        match self {
            Self::Running(stage) => Some(stage),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Running(Run),
    Invalid(QueryInvalid),
    Cancelled,
    Failed(Option<String>),
    ArchiveChanged,
    Finished(Box<QueryResult>),
}

/// The Query tab: the Query text, the running or finished Query and the export of its result.
#[derive(Debug, Default)]
pub struct QueryTab {
    text: String,
    state: State,
    forge: Option<(Weak<EntryIndex>, Arc<ForgeSlot>)>,
    export: Option<ExportJob>,
    completion: CompletionState,
    saved: SavedQueries,
    saved_dialog: Option<SavedDialog>,
    saved_revision: u64,
    edits: Vec<p4kx_settings::SavedQueryEdit>,
    loaded: Option<String>,
}

impl QueryTab {
    /// Returns the Query text of the editor.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the Query text of the editor.
    pub fn set_text(&mut self, text: &str) {
        text.clone_into(&mut self.text);
        self.completion.close();
    }

    /// Returns the text and the completion of the editor for editing.
    pub(crate) const fn editing(&mut self) -> (&mut String, &mut CompletionState) {
        (&mut self.text, &mut self.completion)
    }

    /// Returns the completion of the editor.
    pub(crate) const fn completion(&self) -> &CompletionState {
        &self.completion
    }

    /// Returns the names the editor offers at the cursor, the index of the active one first;
    /// `None` while no list is shown.
    #[must_use]
    pub fn completion_shown(&self) -> Option<(&p4kx_extraction::query::Completion, usize)> {
        self.completion.shown()
    }

    /// Starts reading the completion schema of `index` off the UI thread unless it is read or
    /// being read; `wake` is called once it is read. Together with [`Self::schema`] this is the
    /// one way the tab reaches the schema cache.
    pub(crate) fn prepare_completion(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) {
        self.slot_of(index).prepare_schema(index, wake);
    }

    /// Returns the completion schema of `index` once it was read.
    pub(crate) fn schema(&self, index: &Arc<EntryIndex>) -> Option<Arc<QuerySchema>> {
        let (of, slot) = self.forge.as_ref()?;
        of.upgrade()
            .is_some_and(|of| Arc::ptr_eq(&of, index))
            .then(|| slot.schema())
            .flatten()
    }

    /// Returns whether a Query runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Parses the Query text and runs it over the DataForge of `index` off the UI thread,
    /// replacing the last result; `wake` is called from that thread as it advances and once it
    /// ended. A Query that cannot be parsed is reported at once. Returns whether it started.
    pub(crate) fn run(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> bool {
        if self.is_running() || self.text.trim().is_empty() {
            return false;
        }
        let query = match Query::parse(&self.text) {
            Ok(query) => query,
            Err(error) => {
                self.state = State::Invalid(QueryInvalid::new(self.text.clone(), &error));
                return false;
            }
        };
        let forge = self.slot_of(index);
        let job = Job {
            index: Arc::clone(index),
            query,
            text: self.text.clone(),
            forge,
        };
        self.state = match run::start(job, wake) {
            Ok(run) => State::Running(run),
            Err(reason) => State::Failed(Some(reason)),
        };
        true
    }

    /// Returns the DataForge slot of `index`, a new one when the slot kept is of another
    /// Archive.
    fn slot_of(&mut self, index: &Arc<EntryIndex>) -> Arc<ForgeSlot> {
        if let Some((of, slot)) = &self.forge
            && of.upgrade().is_some_and(|of| Arc::ptr_eq(&of, index))
        {
            return Arc::clone(slot);
        }
        let slot = Arc::new(ForgeSlot::default());
        self.forge = Some((Arc::downgrade(index), Arc::clone(&slot)));
        slot
    }

    /// Returns how many times Queries started reading the DataForge of the open Archive.
    #[must_use]
    pub fn dataforge_reads(&self) -> usize {
        self.forge.as_ref().map_or(0, |(_, slot)| slot.reads())
    }

    /// Returns the bytes of the Query text to mark as an error: the span of the error of the
    /// last run while the text is still the text it was found in.
    #[must_use]
    pub fn mark(&self) -> Option<Range<usize>> {
        match &self.state {
            State::Invalid(invalid) if invalid.query() == self.text => Some(invalid.span()),
            _ => None,
        }
    }

    /// Forgets the result, the running Query, its export and the DataForge read for it,
    /// keeping the Query text, because another Archive was opened.
    pub(crate) fn archive_opened(&mut self) {
        self.state = State::Idle;
        self.forge = None;
        self.export = None;
        self.completion.reset();
    }

    /// Stops the running Query; the tab shows it as cancelled at once.
    pub fn cancel(&mut self) {
        if self.is_running() {
            self.state = State::Cancelled;
        }
    }

    /// Takes the outcome of a Query that ended and of a finished export; returns whether
    /// something changed.
    pub fn poll(&mut self) -> bool {
        let exported = self.export.as_mut().is_some_and(ExportJob::poll);
        let State::Running(run) = &self.state else {
            return exported;
        };
        let Some(outcome) = run.poll() else {
            return exported;
        };
        self.state = match outcome {
            Outcome::Finished(result) => State::Finished(result),
            Outcome::Invalid(invalid) => State::Invalid(invalid),
            Outcome::Failed(reason) => State::Failed(reason),
            Outcome::ArchiveChanged => State::ArchiveChanged,
            Outcome::Cancelled => State::Cancelled,
        };
        true
    }

    /// Returns where the Query stands.
    #[must_use]
    pub fn status(&self) -> QueryStatus<'_> {
        match &self.state {
            State::Idle => QueryStatus::Idle,
            State::Running(run) => QueryStatus::Running(run.stage()),
            State::Invalid(invalid) => QueryStatus::Invalid(invalid),
            State::Cancelled => QueryStatus::Cancelled,
            State::Failed(reason) => QueryStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => QueryStatus::ArchiveChanged,
            State::Finished(result) => QueryStatus::Finished(result),
        }
    }

    /// Returns the finished result, or `None` when there is none.
    #[must_use]
    pub fn result(&self) -> Option<&QueryResult> {
        match &self.state {
            State::Finished(result) => Some(result),
            _ => None,
        }
    }

    /// Returns the finished result for sorting and paging.
    pub fn result_mut(&mut self) -> Option<&mut QueryResult> {
        match &mut self.state {
            State::Finished(result) => Some(result),
            _ => None,
        }
    }

    /// Writes the finished result to `path` in `format` off the UI thread, as the CLI writes
    /// it, calling `wake` from that thread once it ended; returns whether it started.
    pub fn export(
        &mut self,
        path: PathBuf,
        format: QueryExportFormat,
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        if self.is_exporting() {
            return false;
        }
        let Some(result) = self.result() else {
            return false;
        };
        let source = (result.forge(), result.archive());
        let job = export::start(source, result.query(), (path, format), wake);
        self.export = Some(job);
        true
    }

    /// Stops the running export; the file it writes is not created or replaced.
    pub fn cancel_export(&mut self) {
        if let Some(export) = &mut self.export {
            export.cancel();
        }
    }

    /// Returns whether an export runs.
    #[must_use]
    pub fn is_exporting(&self) -> bool {
        self.export.as_ref().is_some_and(ExportJob::is_running)
    }

    /// Returns where the last export stands, or `None` when nothing was exported.
    #[must_use]
    pub fn export_status(&self) -> Option<&QueryExportStatus> {
        self.export.as_ref().map(ExportJob::status)
    }
}
