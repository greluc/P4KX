use std::sync::Arc;

use crate::compare::{Joined, join};
use crate::{Archive, CompareOptions, Comparison, ComparisonError, EntryChange, Levels, Side};

/// A Comparison that owns both Archives, so it can be kept, shared and sent across threads.
///
/// It holds the changes as positions into both Archives and lends them out as
/// [`EntryChange`]s or as a whole [`Comparison`].
#[derive(Debug, Clone)]
pub struct OwnedComparison {
    old: Arc<Archive>,
    new: Arc<Archive>,
    levels: Levels,
    filter: Option<String>,
    changes: Vec<Joined>,
}

impl OwnedComparison {
    /// Compares the `old` with the `new` Archive like [`compare`](crate::compare).
    ///
    /// # Errors
    ///
    /// Returns [`ComparisonError::NoLevel`] when no level is selected and
    /// [`ComparisonError::Cancelled`] when the Comparison was cancelled.
    pub fn compare(
        old: Arc<Archive>,
        new: Arc<Archive>,
        options: &CompareOptions<'_>,
    ) -> Result<Self, ComparisonError> {
        let changes = join(&old, &new, options)?;
        Ok(Self {
            old,
            new,
            levels: options.levels,
            filter: options.filter.map(|filter| filter.as_str().to_owned()),
            changes,
        })
    }

    /// Returns the old Archive.
    #[must_use]
    pub const fn old_archive(&self) -> &Arc<Archive> {
        &self.old
    }

    /// Returns the new Archive.
    #[must_use]
    pub const fn new_archive(&self) -> &Arc<Archive> {
        &self.new
    }

    /// Returns the old side.
    #[must_use]
    pub fn old_side(&self) -> Side<'_> {
        Side::of(&self.old)
    }

    /// Returns the new side.
    #[must_use]
    pub fn new_side(&self) -> Side<'_> {
        Side::of(&self.new)
    }

    /// Returns the number of added, removed and changed Entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.changes.len()
    }

    /// Returns whether no Entry was added, removed or changed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Returns the change at `index` in normalised path order, `None` past the end.
    #[must_use]
    pub fn entry_change(&self, index: usize) -> Option<EntryChange<'_>> {
        self.changes.get(index)?.resolve(&self.old, &self.new)
    }

    /// Returns the Comparison as [`compare`](crate::compare) gives it, for counts and the report.
    #[must_use]
    pub fn comparison(&self) -> Comparison<'_> {
        Comparison {
            old: self.old_side(),
            new: self.new_side(),
            levels: self.levels,
            filter: self.filter.clone(),
            entries: self
                .changes
                .iter()
                .filter_map(|joined| joined.resolve(&self.old, &self.new))
                .collect(),
        }
    }
}
