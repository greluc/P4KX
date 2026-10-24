// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{EntityCatalogue, is_worn_type};
use p4kx_extraction::Guid;

/// What the open choice picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChooserKind {
    /// A piece of armour or clothing: an Entity of a worn item type.
    Piece,
    /// A Loadout: an XML Loadout Entry of the Archive.
    Loadout,
    /// A head: a CHF Entry of the Archive.
    Head,
}

/// What a row of the choice stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// An Entity of the DataForge.
    Entity(Guid),
    /// An Entry of the Archive, by the path the choice proposes it with.
    Path(String),
}

/// One row of the choice: the name, a detail line and what it picks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    name: String,
    detail: String,
    key: Key,
}

impl Item {
    /// Returns the name the row shows first.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the detail the row shows after the name: the item type or the folder.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns what picking the row chooses.
    #[must_use]
    pub const fn key(&self) -> &Key {
        &self.key
    }
}

/// The choice of a piece, a Loadout or a head while it is open: the candidates, narrowed by a
/// search.
#[derive(Debug, Clone)]
pub struct Chooser {
    kind: ChooserKind,
    items: Vec<Item>,
    query: String,
    shown: Vec<u32>,
}

impl Chooser {
    fn new(kind: ChooserKind, items: Vec<Item>) -> Self {
        let mut chooser = Self {
            kind,
            items,
            query: String::new(),
            shown: Vec::new(),
        };
        chooser.refilter();
        chooser
    }

    pub(super) fn pieces(catalogue: &EntityCatalogue) -> Self {
        let items = catalogue
            .entries()
            .iter()
            .filter(|entry| is_worn_type(entry.facts().item_type()))
            .map(|entry| Item {
                name: entry.class_name().to_owned(),
                detail: entry.facts().item_type().to_owned(),
                key: Key::Entity(entry.guid()),
            })
            .collect();
        Self::new(ChooserKind::Piece, items)
    }

    pub(super) fn loadouts(paths: &[String]) -> Self {
        Self::new(
            ChooserKind::Loadout,
            items_of(paths, Some("scripts/loadouts/")),
        )
    }

    pub(super) fn heads(paths: &[String]) -> Self {
        Self::new(ChooserKind::Head, items_of(paths, None))
    }

    fn refilter(&mut self) {
        let terms: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        self.shown = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                terms
                    .iter()
                    .all(|term| contains(&item.name, term) || contains(&item.detail, term))
            })
            .filter_map(|(at, _)| u32::try_from(at).ok())
            .collect();
    }

    /// Returns what the choice picks.
    #[must_use]
    pub const fn kind(&self) -> ChooserKind {
        self.kind
    }

    /// Returns the search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Searches the names and details; every word has to occur, ignoring ASCII case.
    pub fn set_query(&mut self, text: &str) {
        text.clone_into(&mut self.query);
        self.refilter();
    }

    /// Returns how many candidates there are, whatever the search says.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.items.len()
    }

    /// Returns how many candidates the search leaves.
    #[must_use]
    pub const fn shown_len(&self) -> usize {
        self.shown.len()
    }

    /// Returns the candidate at `position` of the shown ones.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&Item> {
        let at = usize::try_from(*self.shown.get(position)?).ok()?;
        self.items.get(at)
    }
}

fn items_of(paths: &[String], below: Option<&str>) -> Vec<Item> {
    paths
        .iter()
        .map(|path| {
            let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
            let detail = below
                .and_then(|prefix| {
                    folder
                        .get(..prefix.len())
                        .filter(|start| start.eq_ignore_ascii_case(prefix))
                        .and_then(|_| folder.get(prefix.len()..))
                })
                .unwrap_or(folder);
            Item {
                name: name.to_owned(),
                detail: detail.to_owned(),
                key: Key::Path(path.clone()),
            }
        })
        .collect()
}

fn contains(haystack: &str, term: &str) -> bool {
    let (haystack, term) = (haystack.as_bytes(), term.as_bytes());
    term.is_empty()
        || haystack
            .windows(term.len())
            .any(|window| window.eq_ignore_ascii_case(term))
}

#[cfg(test)]
mod tests {
    use super::{Chooser, Key};

    fn paths() -> Vec<String> {
        [
            "Scripts/Loadouts/Character/Grunt.xml",
            "Scripts/Loadouts/Ships/Cutlass.xml",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn a_loadout_shows_its_folder_below_the_loadouts() {
        let chooser = Chooser::loadouts(&paths());

        let first = chooser.get(0);

        assert_eq!(first.map(|item| item.name()), Some("Grunt.xml"), "name");
        assert_eq!(first.map(|item| item.detail()), Some("Character"), "folder");
        assert_eq!(
            first.map(|item| item.key().clone()),
            Some(Key::Path("Scripts/Loadouts/Character/Grunt.xml".to_owned())),
            "the path the resolver reads"
        );
    }

    #[test]
    fn the_search_needs_every_word_in_the_name_or_the_detail() {
        let mut chooser = Chooser::loadouts(&paths());

        chooser.set_query("SHIPS cut");

        assert_eq!(chooser.shown_len(), 1, "one left");
        assert_eq!(chooser.total(), 2, "of two");
        chooser.set_query("none");
        assert_eq!(chooser.shown_len(), 0, "no match");
    }
}
