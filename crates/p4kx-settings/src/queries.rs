use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// A Query kept under a name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedQuery {
    name: String,
    text: String,
}

impl SavedQuery {
    /// Returns the name the Query is kept under.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the Query text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Why a Query cannot be saved or renamed under a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NameProblem {
    /// The name is empty or only whitespace.
    #[error("the name is empty")]
    Empty,
    /// The name is longer than [`SavedQueries::MAX_NAME_LEN`] characters.
    #[error("the name is longer than {} characters", SavedQueries::MAX_NAME_LEN)]
    TooLong,
    /// Another saved Query has the name.
    #[error("another saved Query has the name")]
    Taken,
    /// The name holds a control character.
    #[error("the name holds a control character")]
    Control,
    /// [`SavedQueries::LIMIT`] Queries or [`SavedQueries::MAX_TOTAL_LEN`] bytes of names and
    /// texts are saved already.
    #[error(
        "at most {} Queries and {} bytes can be saved",
        SavedQueries::LIMIT,
        SavedQueries::MAX_TOTAL_LEN
    )]
    Full,
    /// The Query text is empty, longer than [`SavedQueries::MAX_TEXT_LEN`] bytes or holds a
    /// control character other than tab and line breaks.
    #[error("the Query text is empty, too long or holds a control character")]
    BadText,
}

/// One change to the saved Queries by name, replayed onto the list as another window may have
/// stored it meanwhile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedQueryEdit {
    /// Saves `text` as `name`, replacing a Query of that name.
    Save {
        /// The name.
        name: String,
        /// The Query text.
        text: String,
    },
    /// Renames the Query `from` to `to`.
    Rename {
        /// The old name.
        from: String,
        /// The new name.
        to: String,
    },
    /// Deletes the Query `name`.
    Delete {
        /// The name.
        name: String,
    },
}

/// The saved Queries in the order they were saved.
///
/// Names are unique, trimmed and free of control characters. Read from a file, an entry that is
/// no saved Query, has no valid name or text, or repeats a name is dropped, and so is every entry
/// beyond [`Self::LIMIT`] or [`Self::MAX_TOTAL_LEN`]; a value that is no list reads as no saved
/// Query.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SavedQueries(Vec<SavedQuery>);

impl SavedQueries {
    /// How many Queries can be saved.
    pub const LIMIT: usize = 200;
    /// The longest name in characters.
    pub const MAX_NAME_LEN: usize = 120;
    /// The longest Query text in bytes, the longest Query.
    pub const MAX_TEXT_LEN: usize = 65_536;
    /// The most bytes of names and texts saved together, a quarter of the settings file cap so
    /// that escaping never lets the file outgrow it.
    pub const MAX_TOTAL_LEN: usize = 4 * 1024 * 1024;

    /// Returns the saved Queries.
    pub fn iter(&self) -> std::slice::Iter<'_, SavedQuery> {
        self.0.iter()
    }

    /// Returns how many Queries are saved.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether no Query is saved.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the saved Query at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&SavedQuery> {
        self.0.get(index)
    }

    /// Returns the position of the Query saved as `name`, after trimming it.
    #[must_use]
    pub fn position(&self, name: &str) -> Option<usize> {
        let name = name.trim();
        self.0.iter().position(|saved| saved.name == name)
    }

    /// Returns why `text` cannot be saved as `name`; saving under a taken name replaces that
    /// Query's text, so only a full list stops it.
    #[must_use]
    pub fn check_save(&self, name: &str, text: &str) -> Option<NameProblem> {
        if !valid_text(text) {
            return Some(NameProblem::BadText);
        }
        if let Some(problem) = name_problem(name) {
            return Some(problem);
        }
        let replaced = self.position(name).and_then(|index| self.0.get(index));
        let total = self
            .total_len()
            .saturating_sub(replaced.map_or(0, |saved| saved.name.len() + saved.text.len()))
            .saturating_add(name.trim().len() + text.len());
        let full =
            (replaced.is_none() && self.0.len() >= Self::LIMIT) || total > Self::MAX_TOTAL_LEN;
        full.then_some(NameProblem::Full)
    }

    /// Returns the bytes of all names and texts together.
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.0
            .iter()
            .map(|saved| saved.name.len() + saved.text.len())
            .sum()
    }

    /// Carries out `edit` by name; returns whether the list changed. An edit that no longer
    /// fits, e.g. a rename to a name taken meanwhile, is left out.
    pub fn apply(&mut self, edit: &SavedQueryEdit) -> bool {
        match edit {
            SavedQueryEdit::Save { name, text } => self.save(name, text).is_ok(),
            SavedQueryEdit::Rename { from, to } => self
                .position(from)
                .is_some_and(|index| self.rename(index, to).is_ok()),
            SavedQueryEdit::Delete { name } => {
                self.position(name).is_some_and(|index| self.remove(index))
            }
        }
    }

    /// Saves `text` as `name`, replacing the text of a Query of that name; returns its position.
    ///
    /// # Errors
    ///
    /// Returns the [`NameProblem`] of [`Self::check_save`].
    pub fn save(&mut self, name: &str, text: &str) -> Result<usize, NameProblem> {
        if let Some(problem) = self.check_save(name, text) {
            return Err(problem);
        }
        if let Some(index) = self.position(name)
            && let Some(saved) = self.0.get_mut(index)
        {
            text.clone_into(&mut saved.text);
            return Ok(index);
        }
        self.0.push(SavedQuery {
            name: name.trim().to_owned(),
            text: text.to_owned(),
        });
        Ok(self.0.len() - 1)
    }

    /// Returns why the Query at `index` cannot be renamed to `name`.
    #[must_use]
    pub fn check_rename(&self, index: usize, name: &str) -> Option<NameProblem> {
        name_problem(name).or_else(|| {
            self.position(name)
                .is_some_and(|other| other != index)
                .then_some(NameProblem::Taken)
        })
    }

    /// Renames the Query at `index` to `name`.
    ///
    /// # Errors
    ///
    /// Returns the [`NameProblem`] of [`Self::check_rename`]; an `index` beyond the list is
    /// ignored.
    pub fn rename(&mut self, index: usize, name: &str) -> Result<(), NameProblem> {
        if let Some(problem) = self.check_rename(index, name) {
            return Err(problem);
        }
        if let Some(saved) = self.0.get_mut(index) {
            name.trim().clone_into(&mut saved.name);
        }
        Ok(())
    }

    /// Removes the Query at `index`; returns whether there was one.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.0.len() {
            self.0.remove(index);
            true
        } else {
            false
        }
    }
}

fn name_problem(name: &str) -> Option<NameProblem> {
    let name = name.trim();
    if name.is_empty() {
        Some(NameProblem::Empty)
    } else if name.chars().count() > SavedQueries::MAX_NAME_LEN {
        Some(NameProblem::TooLong)
    } else if name.chars().any(char::is_control) {
        Some(NameProblem::Control)
    } else {
        None
    }
}

fn valid_text(text: &str) -> bool {
    !text.trim().is_empty()
        && text.len() <= SavedQueries::MAX_TEXT_LEN
        && !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Stored {
    List(Vec<Entry>),
    Other(IgnoredAny),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Query { name: String, text: String },
    Other(IgnoredAny),
}

impl<'de> Deserialize<'de> for SavedQueries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let Stored::List(entries) = Stored::deserialize(deserializer)? else {
            return Ok(Self::default());
        };
        let mut queries = Self::default();
        for entry in entries {
            if queries.len() == Self::LIMIT {
                break;
            }
            if let Entry::Query { name, text } = entry
                && name_problem(&name).is_none()
                && valid_text(&text)
                && queries.position(&name).is_none()
                && queries.total_len() + name.trim().len() + text.len() <= Self::MAX_TOTAL_LEN
            {
                queries.0.push(SavedQuery {
                    name: name.trim().to_owned(),
                    text,
                });
            }
        }
        Ok(queries)
    }
}

impl<'a> IntoIterator for &'a SavedQueries {
    type Item = &'a SavedQuery;
    type IntoIter = std::slice::Iter<'a, SavedQuery>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
