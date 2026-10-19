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
    /// [`SavedQueries::LIMIT`] Queries are saved already.
    #[error("{} Queries are saved already", SavedQueries::LIMIT)]
    Full,
    /// The Query text is empty or longer than [`SavedQueries::MAX_TEXT_LEN`] bytes.
    #[error("the Query text is empty or too long")]
    BadText,
}

/// The saved Queries in the order they were saved.
///
/// Names are unique and trimmed. Read from a file, an entry that is no saved Query, has no
/// valid name or text, or repeats a name is dropped, and so is every entry beyond
/// [`Self::LIMIT`]; a value that is no list reads as no saved Query.
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
        (self.position(name).is_none() && self.0.len() >= Self::LIMIT).then_some(NameProblem::Full)
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
    } else {
        None
    }
}

fn valid_text(text: &str) -> bool {
    !text.trim().is_empty() && text.len() <= SavedQueries::MAX_TEXT_LEN
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
