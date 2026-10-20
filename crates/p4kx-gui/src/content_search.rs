use std::sync::Arc;
use std::time::Duration;

use p4kx_search::{
    EntryHit, LocalizationHit, RecordHit, SearchReport, SearchStats, Source, Sources, Unavailable,
};

mod run;

use crate::EntryIndex;
use run::{Outcome, Run};

/// What a finished content search found.
#[derive(Debug)]
pub struct ContentResult {
    text: String,
    report: SearchReport,
    elapsed: Duration,
}

impl ContentResult {
    /// Returns the text that was searched for.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the matches in Records, in Record order.
    #[must_use]
    pub fn records(&self) -> &[RecordHit] {
        &self.report.records
    }

    /// Returns the matches in localisation texts, by language and line.
    #[must_use]
    pub fn localization(&self) -> &[LocalizationHit] {
        &self.report.localization
    }

    /// Returns the matches in Entries, in path order.
    #[must_use]
    pub fn entries(&self) -> &[EntryHit] {
        &self.report.entries
    }

    /// Returns the number of matches of `source`.
    #[must_use]
    pub const fn len(&self, source: Source) -> usize {
        match source {
            Source::Records => self.report.records.len(),
            Source::Localization => self.report.localization.len(),
            Source::Entries => self.report.entries.len(),
        }
    }

    /// Returns whether there is no match at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.report.is_empty()
    }

    /// Returns whether `source` holds more matches than were kept.
    #[must_use]
    pub const fn has_more(&self, source: Source) -> bool {
        self.report.has_more.contains(source)
    }

    /// Returns the sources that had nothing to search in.
    #[must_use]
    pub fn unavailable(&self) -> &[Unavailable] {
        &self.report.unavailable
    }

    /// Returns what was read and what was left out.
    #[must_use]
    pub const fn stats(&self) -> SearchStats {
        self.report.stats
    }

    /// Returns how long the search took.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Where the search of the tab stands.
#[derive(Debug, Clone, Copy)]
pub enum ContentSearchStatus<'a> {
    /// Nothing was searched yet.
    Idle,
    /// A search runs off the UI thread.
    Running,
    /// It was cancelled.
    Cancelled,
    /// It failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the search was not finished.
    ArchiveChanged,
    /// It finished.
    Finished(&'a ContentResult),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Running(Run),
    Cancelled,
    Failed(Option<String>),
    ArchiveChanged,
    Finished(Box<ContentResult>),
}

/// The Search tab: the text, the sources to search, the running or finished search and the list
/// of hits shown.
#[derive(Debug)]
pub struct ContentSearchTab {
    text: String,
    sources: Sources,
    state: State,
    shown: Source,
    selected: Option<usize>,
    serial: u64,
}

impl Default for ContentSearchTab {
    fn default() -> Self {
        Self {
            text: String::new(),
            sources: Sources::ALL,
            state: State::Idle,
            shown: Source::Records,
            selected: None,
            serial: 0,
        }
    }
}

impl ContentSearchTab {
    /// Returns the text of the search field.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the text of the search field for editing.
    pub const fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Sets the text of the search field.
    pub fn set_text(&mut self, text: &str) {
        text.clone_into(&mut self.text);
    }

    /// Returns the sources the next search looks at.
    #[must_use]
    pub const fn sources(&self) -> Sources {
        self.sources
    }

    /// Adds `source` to the sources of the next search or removes it.
    pub fn set_source(&mut self, source: Source, on: bool) {
        self.sources = if on {
            self.sources.with(source)
        } else {
            Source::ALL
                .into_iter()
                .filter(|other| *other != source && self.sources.contains(*other))
                .collect()
        };
    }

    /// Returns whether a search runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Returns where the search stands.
    #[must_use]
    pub fn status(&self) -> ContentSearchStatus<'_> {
        match &self.state {
            State::Idle => ContentSearchStatus::Idle,
            State::Running(_) => ContentSearchStatus::Running,
            State::Cancelled => ContentSearchStatus::Cancelled,
            State::Failed(reason) => ContentSearchStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => ContentSearchStatus::ArchiveChanged,
            State::Finished(result) => ContentSearchStatus::Finished(result),
        }
    }

    /// Returns the finished result, or `None` when there is none.
    #[must_use]
    pub fn result(&self) -> Option<&ContentResult> {
        match &self.state {
            State::Finished(result) => Some(result),
            _ => None,
        }
    }

    /// Returns the source whose hits are listed.
    #[must_use]
    pub const fn shown(&self) -> Source {
        self.shown
    }

    /// Lists the hits of `source`.
    pub const fn show(&mut self, source: Source) {
        if self.shown as u8 != source as u8 {
            self.shown = source;
            self.selected = None;
            self.serial += 1;
        }
    }

    /// Returns the position of the selected hit among those of the listed source.
    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Selects the hit at `position` among those of the listed source.
    pub const fn select(&mut self, position: usize) {
        self.selected = Some(position);
    }

    /// Returns a number that changes whenever another list is shown, for caches of laid-out rows.
    #[must_use]
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Searches the open Archive of `index` for the text of the field in the chosen sources off
    /// the UI thread, replacing the last result; `wake` is called from the thread once it ended.
    /// Returns whether it started: nothing starts for an empty text, without a source or while a
    /// search runs.
    pub(crate) fn run(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> bool {
        if self.is_running() || self.text.is_empty() || self.sources.is_empty() {
            return false;
        }
        self.state = match run::start(index, &self.text, self.sources, wake) {
            Ok(run) => State::Running(run),
            Err(reason) => State::Failed(Some(reason)),
        };
        self.selected = None;
        self.serial += 1;
        true
    }

    /// Stops the running search; the tab shows it as cancelled at once.
    pub fn cancel(&mut self) {
        if self.is_running() {
            self.state = State::Cancelled;
        }
    }

    /// Forgets the result and the running search, keeping the text and the sources, because
    /// another Archive was opened.
    pub(crate) fn archive_opened(&mut self) {
        self.state = State::Idle;
        self.selected = None;
        self.serial += 1;
    }

    /// Takes the outcome of a search that ended; returns whether something changed.
    pub fn poll(&mut self) -> bool {
        let State::Running(run) = &self.state else {
            return false;
        };
        let Some(outcome) = run.poll() else {
            return false;
        };
        self.serial += 1;
        self.state = match outcome {
            Outcome::Finished(result) => {
                self.shown = first_with_hits(&result, self.shown);
                State::Finished(result)
            }
            Outcome::Failed(reason) => State::Failed(reason),
            Outcome::ArchiveChanged => State::ArchiveChanged,
            Outcome::Cancelled => State::Cancelled,
        };
        true
    }
}

/// Returns `current` when it has hits, otherwise the first source that has some.
fn first_with_hits(result: &ContentResult, current: Source) -> Source {
    if result.len(current) > 0 {
        return current;
    }
    Source::ALL
        .into_iter()
        .find(|source| result.len(*source) > 0)
        .unwrap_or(current)
}
