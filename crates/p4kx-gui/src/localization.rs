use std::sync::Arc;

use p4kx_extraction::Language as GameLanguage;

mod filter;
mod load;
mod table;

pub use table::LocalizationTable;

use crate::EntryIndex;
use filter::Needle;
use load::{Load, Outcome, Search};

/// Where the localisation table of the tab stands.
#[derive(Debug, Clone, Copy)]
pub enum LocalizationStatus<'a> {
    /// The texts were not asked for yet.
    Idle,
    /// The `global.ini` files are read off the UI thread.
    Loading,
    /// The Archive holds no `global.ini`.
    NoTexts,
    /// Reading failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the texts were not read.
    ArchiveChanged,
    /// The table is read.
    Ready(&'a LocalizationTable),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Loading(Load),
    Failed(Option<String>),
    ArchiveChanged,
    Ready(Box<Loaded>),
}

#[derive(Debug)]
enum Shown {
    All,
    Rows(Vec<u32>),
}

#[derive(Debug)]
struct Loaded {
    table: Arc<LocalizationTable>,
    shown: Shown,
    search: Option<Search>,
    selected: Option<(u32, Option<GameLanguage>)>,
    generation: u64,
    table_serial: u64,
}

impl Loaded {
    fn drop_hidden_selection(&mut self) {
        if let (Shown::Rows(rows), Some((row, _))) = (&self.shown, self.selected)
            && rows.binary_search(&row).is_err()
        {
            self.selected = None;
        }
    }
}

/// The localisation tab: the table of every key with its text in every language of the open
/// Archive, the search text and the selected cell.
#[derive(Debug, Default)]
pub struct LocalizationTab {
    state: State,
    query: String,
    generation: u64,
    tables: u64,
}

impl LocalizationTab {
    /// Returns the search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns where the table stands.
    #[must_use]
    pub fn status(&self) -> LocalizationStatus<'_> {
        match &self.state {
            State::Idle => LocalizationStatus::Idle,
            State::Loading(_) => LocalizationStatus::Loading,
            State::Failed(reason) => LocalizationStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => LocalizationStatus::ArchiveChanged,
            State::Ready(loaded) if loaded.table.languages().is_empty() => {
                LocalizationStatus::NoTexts
            }
            State::Ready(loaded) => LocalizationStatus::Ready(&loaded.table),
        }
    }

    /// Returns whether the texts are read or being read.
    #[must_use]
    pub const fn is_started(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    /// Starts reading the `global.ini` files of `index` off the UI thread, calling `wake` from
    /// it once it ended; returns whether it started. Nothing starts when the texts are read
    /// or being read.
    pub(crate) fn load(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        if self.is_started() {
            return false;
        }
        self.state = match load::start(index, wake) {
            Ok(load) => State::Loading(load),
            Err(reason) => State::Failed(Some(reason)),
        };
        true
    }

    /// Forgets the table, the selection and the search text, because another Archive was
    /// opened.
    pub(crate) fn archive_opened(&mut self) {
        self.state = State::Idle;
        self.query.clear();
    }

    /// Takes the table once it is read and the rows of a search that ended; returns whether
    /// something changed.
    pub fn poll(&mut self) -> bool {
        match &mut self.state {
            State::Loading(load) => {
                let Some(outcome) = load.poll() else {
                    return false;
                };
                self.generation += 1;
                self.tables += 1;
                self.state = match outcome {
                    Outcome::Ready(table) => State::Ready(Box::new(Loaded {
                        table,
                        shown: Shown::All,
                        search: None,
                        selected: None,
                        generation: self.generation,
                        table_serial: self.tables,
                    })),
                    Outcome::Failed(reason) => State::Failed(reason),
                    Outcome::ArchiveChanged => State::ArchiveChanged,
                };
                true
            }
            State::Ready(loaded) => {
                let Some(rows) = loaded.search.as_ref().and_then(Search::poll) else {
                    return false;
                };
                self.generation += 1;
                loaded.generation = self.generation;
                loaded.search = None;
                loaded.shown = Shown::Rows(rows);
                loaded.drop_hidden_selection();
                true
            }
            _ => false,
        }
    }

    /// Sets the search text and finds the rows whose key or text in any language contains it,
    /// ignoring case, off the UI thread; `wake` is called from it once they are known. The
    /// rows of the previous text stay until then.
    pub fn set_query(&mut self, query: &str, wake: impl Fn() + Send + 'static) {
        if self.query == query {
            return;
        }
        query.clone_into(&mut self.query);
        let State::Ready(loaded) = &mut self.state else {
            return;
        };
        loaded.search = None;
        let Some(needle) = Needle::new(query) else {
            self.generation += 1;
            loaded.generation = self.generation;
            loaded.shown = Shown::All;
            return;
        };
        match load::search(&loaded.table, &needle, wake) {
            Ok(search) => loaded.search = Some(search),
            Err(rows) => {
                self.generation += 1;
                loaded.generation = self.generation;
                loaded.shown = Shown::Rows(rows);
                loaded.drop_hidden_selection();
            }
        }
    }

    /// Returns whether a search is still running.
    #[must_use]
    pub fn is_searching(&self) -> bool {
        matches!(&self.state, State::Ready(loaded) if loaded.search.is_some())
    }

    /// Returns the number of rows shown.
    #[must_use]
    pub fn shown_len(&self) -> usize {
        match &self.state {
            State::Ready(loaded) => match &loaded.shown {
                Shown::All => loaded.table.len(),
                Shown::Rows(rows) => rows.len(),
            },
            _ => 0,
        }
    }

    /// Returns the table row at `position` among the rows shown.
    #[must_use]
    pub fn row_at(&self, position: usize) -> Option<u32> {
        let State::Ready(loaded) = &self.state else {
            return None;
        };
        match &loaded.shown {
            Shown::All => u32::try_from(position)
                .ok()
                .filter(|_| position < loaded.table.len()),
            Shown::Rows(rows) => rows.get(position).copied(),
        }
    }

    /// Returns a number that changes whenever the rows shown or the table change, to tell a
    /// cache of laid-out rows when to start over.
    #[must_use]
    pub fn serial(&self) -> u64 {
        match &self.state {
            State::Ready(loaded) => loaded.generation,
            _ => 0,
        }
    }

    /// Returns a number that changes whenever another table was read, to tell a cache of laid-out
    /// heads when to start over.
    #[must_use]
    pub fn table_serial(&self) -> u64 {
        match &self.state {
            State::Ready(loaded) => loaded.table_serial,
            _ => 0,
        }
    }

    /// Selects the cell of the table row `row` in `language`, or the key cell for `None`.
    pub fn select(&mut self, row: u32, language: Option<GameLanguage>) {
        if let State::Ready(loaded) = &mut self.state {
            loaded.selected = Some((row, language));
        }
    }

    /// Returns the selected cell: its table row and language, `None` for the key cell.
    #[must_use]
    pub fn selected(&self) -> Option<(u32, Option<GameLanguage>)> {
        match &self.state {
            State::Ready(loaded) => loaded.selected,
            _ => None,
        }
    }

    /// Returns the key of the selected row, to copy.
    #[must_use]
    pub fn selected_key(&self) -> Option<&str> {
        let State::Ready(loaded) = &self.state else {
            return None;
        };
        loaded.table.key(loaded.selected?.0)
    }

    /// Returns the text of the selected cell with its line breaks decoded, to copy; `None`
    /// without a selection or when the cell has no translation.
    #[must_use]
    pub fn selected_text(&self) -> Option<String> {
        let State::Ready(loaded) = &self.state else {
            return None;
        };
        let (row, language) = loaded.selected?;
        loaded.table.decoded_text(row, language?)
    }
}
