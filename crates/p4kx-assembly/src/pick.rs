// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The alternative an export takes from each group of a generated Loadout.

use crate::ChangeError;

/// The alternative to take from the groups of the assortments of a generated Loadout.
///
/// The game picks one alternative per group at random; without a pick a group gives its first
/// alternative. Indices count from 0, so index 0 is the first alternative. A group is named by
/// its `Group` field in the assortment, matched ignoring ASCII case, in whichever assortment it
/// occurs; a pick of a group that no assortment of the Loadout has changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadoutPicks {
    every: Option<usize>,
    groups: Vec<(String, usize)>,
}

/// What the picks say about one group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Choice {
    /// The group is named: an index beyond its alternatives is an issue.
    Named(usize),
    /// Every group is picked: a group that lacks the index keeps its first alternative.
    Every(usize),
    /// Nothing is picked: the first alternative.
    First,
}

impl LoadoutPicks {
    /// Creates picks that leave every group at its first alternative.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            every: None,
            groups: Vec::new(),
        }
    }

    /// Takes the alternative `index` of every group that has it; the other groups keep their
    /// first alternative, which is no issue.
    #[must_use]
    pub const fn every(mut self, index: usize) -> Self {
        self.every = Some(index);
        self
    }

    /// Takes the alternative `index` of the group `group`, replacing an earlier pick of it. An
    /// index beyond the group is an issue of the Loadout and the group keeps its first
    /// alternative.
    #[must_use]
    pub fn group(mut self, group: impl Into<String>, index: usize) -> Self {
        let group = group.into();
        self.groups
            .retain(|(name, _)| !name.eq_ignore_ascii_case(&group));
        self.groups.push((group, index));
        self
    }

    /// Adds the pick `text`, either `<group>=<index>` or a bare `<index>` for every group.
    ///
    /// # Errors
    ///
    /// Returns [`ChangeError::InvalidPick`] when `text` is neither form, or names no group.
    pub fn add_text(self, text: &str) -> Result<Self, ChangeError> {
        let invalid = || ChangeError::InvalidPick {
            text: text.to_owned(),
        };
        match text.split_once('=') {
            Some((group, index)) => {
                let group = group.trim();
                let index = index.trim().parse().map_err(|_| invalid())?;
                if group.is_empty() {
                    return Err(invalid());
                }
                Ok(self.group(group, index))
            }
            None => Ok(self.every(text.trim().parse().map_err(|_| invalid())?)),
        }
    }

    /// Reads picks from `text`, each `<group>=<index>` or a bare `<index>` as for
    /// [`add_text`](Self::add_text), separated by commas; blank parts are skipped.
    ///
    /// # Errors
    ///
    /// Returns [`ChangeError::InvalidPick`] for the first part that is not a pick.
    pub fn from_list(text: &str) -> Result<Self, ChangeError> {
        text.split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .try_fold(Self::new(), Self::add_text)
    }

    /// Returns whether no group is picked.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.every.is_none() && self.groups.is_empty()
    }

    /// Returns the index picked for every group, if any.
    #[must_use]
    pub const fn every_index(&self) -> Option<usize> {
        self.every
    }

    /// Returns the groups picked by name as `(group, index)`, as given.
    #[must_use]
    pub fn groups(&self) -> &[(String, usize)] {
        &self.groups
    }

    /// Returns what the picks say about `group`; a named pick wins over the pick of every group.
    pub(crate) fn choice(&self, group: &str) -> Choice {
        self.groups
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(group))
            .map(|&(_, index)| Choice::Named(index))
            .or_else(|| self.every.map(Choice::Every))
            .unwrap_or(Choice::First)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_a_group_pick_or_a_bare_index() {
        let picks = LoadoutPicks::new()
            .add_text("Body=2")
            .and_then(|picks| picks.add_text(" 1 "));

        assert_eq!(
            picks,
            Ok(LoadoutPicks::new().group("Body", 2).every(1)),
            "the group name is trimmed, the bare index is every group"
        );
    }

    #[test]
    fn a_list_is_comma_separated_and_skips_blank_parts() {
        assert_eq!(
            LoadoutPicks::from_list("Body=2, ,Egg=1,"),
            Ok(LoadoutPicks::new().group("Body", 2).group("Egg", 1))
        );
        assert_eq!(LoadoutPicks::from_list("  "), Ok(LoadoutPicks::new()));
        assert!(LoadoutPicks::from_list("Body=2, x").is_err());
    }

    #[test]
    fn malformed_text_is_invalid() {
        for text in ["", "Body=", "=1", "Body=x", "x", "Body=-1", "1=2=3"] {
            assert_eq!(
                LoadoutPicks::new().add_text(text),
                Err(ChangeError::InvalidPick {
                    text: text.to_owned()
                }),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_named_pick_wins_over_every_and_ignores_case() {
        let picks = LoadoutPicks::new().every(1).group("Body", 2);

        assert_eq!(picks.choice("body"), Choice::Named(2));
        assert_eq!(picks.choice("Eggs"), Choice::Every(1));
        assert_eq!(LoadoutPicks::new().choice("Body"), Choice::First);
    }

    #[test]
    fn a_later_pick_of_a_group_replaces_the_earlier() {
        let picks = LoadoutPicks::new().group("Body", 1).group("BODY", 3);

        assert_eq!(picks.groups(), [("BODY".to_owned(), 3)]);
    }
}
