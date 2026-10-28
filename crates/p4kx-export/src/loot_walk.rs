// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The walk through the containers that entities of containers place.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use p4kx_extraction::{Archive, CancelToken, Guid};
use rayon::prelude::*;

use crate::container_read::read_limited;
use crate::loot_entxml::{self, EntityLink};
use crate::loot_unreadable::Unreadable;
use crate::starmap_index::{Index, key};
use crate::starmap_walk::Opened;

/// The most distinct containers followed from the system containers.
pub(crate) const MAX_CONTAINERS: usize = 1 << 18;

/// The largest property file of one entity that is read, 1 MiB.
const MAX_ENTXML_LEN: u64 = 1 << 20;

/// The folder of a container that holds the property file of each entity.
const ENTITY_FOLDER: &str = "entdata/";

/// The extension of the property file of an entity.
const ENTITY_EXTENSION: &str = "entxml";

/// An entity of a container that carries a multi config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Placed {
    pub(crate) config: Guid,
    /// The path of the placed container: the Archive's when it has it, else as written.
    pub(crate) path: String,
    /// The Archive entry of the placed container, `None` when the Archive lacks it.
    pub(crate) entry: Option<usize>,
}

/// What the entities of one container say: the containers they place and the configs they carry.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Summary {
    pub(crate) nested: Vec<usize>,
    pub(crate) placed: Vec<Placed>,
    pub(crate) unreadable: Option<Unreadable>,
    /// The entity files that name several different containers or configs, of which only the
    /// first of each is used.
    pub(crate) several: Vec<String>,
}

/// The containers read, by Archive entry, and how many distinct containers the limit left unread.
pub(crate) struct Explored {
    pub(crate) summaries: HashMap<usize, Summary>,
    pub(crate) beyond_limit: usize,
}

/// Reads the containers in `opened` and every container their entities place, breadth first,
/// each once, until none is left or [`MAX_CONTAINERS`] are read.
pub(crate) fn explore(
    archive: &Archive,
    index: &Index<'_>,
    opened: &[Opened],
    cancel: Option<&CancelToken>,
) -> Explored {
    explore_within(archive, index, opened, cancel, MAX_CONTAINERS)
}

fn explore_within(
    archive: &Archive,
    index: &Index<'_>,
    opened: &[Opened],
    cancel: Option<&CancelToken>,
    limit: usize,
) -> Explored {
    let mut summaries: HashMap<usize, Summary> = HashMap::new();
    let mut unread: HashSet<usize> = HashSet::new();
    let mut frontier: Vec<usize> = opened.iter().map(|root| root.entry).collect();
    while !frontier.is_empty() && !cancel.is_some_and(CancelToken::is_cancelled) {
        frontier.sort_unstable();
        frontier.dedup();
        frontier.retain(|entry| !summaries.contains_key(entry));
        let room = limit.saturating_sub(summaries.len());
        if frontier.len() > room {
            unread.extend(frontier.split_off(room));
        }
        let read: Vec<(usize, Summary)> = frontier
            .par_iter()
            .filter(|_| !cancel.is_some_and(CancelToken::is_cancelled))
            .map(|&entry| (entry, summarise(archive, index, entry)))
            .collect();
        frontier = read
            .iter()
            .flat_map(|(_, summary)| summary.nested.iter().copied())
            .collect();
        summaries.extend(read);
    }
    Explored {
        summaries,
        beyond_limit: unread.len(),
    }
}

fn summarise(archive: &Archive, index: &Index<'_>, entry: usize) -> Summary {
    let mut summary = Summary::default();
    let Some(socpak) = archive.entries().get(entry) else {
        summary.unreadable = Some(Unreadable::NoSuchEntry);
        return summary;
    };
    let Ok(container) = archive.open_container(socpak) else {
        summary.unreadable = Some(Unreadable::NotAContainer);
        return summary;
    };
    for inner in container
        .entries()
        .iter()
        .filter(|inner| is_entity_file(inner.path()))
    {
        let Some(bytes) = read_limited(&container, inner, MAX_ENTXML_LEN) else {
            summary.unreadable = Some(Unreadable::EntityFile);
            continue;
        };
        let EntityLink {
            container: written,
            config,
            several,
        } = loot_entxml::read(&bytes);
        if several {
            summary.several.push(inner.path().to_owned());
        }
        let found = written.as_deref().and_then(|path| index.find(path));
        if let Some(found) = found {
            summary.nested.push(found);
        }
        if let Some(config) = config {
            let path = match (found, written) {
                (Some(found), _) => path_of(archive, found).to_owned(),
                (None, written) => written.unwrap_or_default(),
            };
            summary.placed.push(Placed {
                config,
                path,
                entry: found,
            });
        }
    }
    summary.nested.sort_unstable();
    summary.nested.dedup();
    summary
}

fn is_entity_file(path: &str) -> bool {
    key(path).replace('\\', "/").contains(ENTITY_FOLDER)
        && Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(ENTITY_EXTENSION))
}

/// Returns every [`Placed`] in `root` and the containers below it, each container once; a cycle
/// of containers ends where it returns to a container already seen. The cost is the size of the
/// subgraph below `root`, once per distinct root.
pub(crate) fn below(root: usize, summaries: &HashMap<usize, Summary>) -> Vec<&Placed> {
    let mut seen: HashSet<usize> = HashSet::from([root]);
    let mut pending = vec![root];
    let mut found = Vec::new();
    while let Some(entry) = pending.pop() {
        let Some(summary) = summaries.get(&entry) else {
            continue;
        };
        found.extend(&summary.placed);
        pending.extend(summary.nested.iter().filter(|&&next| seen.insert(next)));
    }
    found
}

/// Returns the path of the Archive entry `entry`, empty for none.
pub(crate) fn path_of(archive: &Archive, entry: usize) -> &str {
    archive
        .entries()
        .get(entry)
        .map_or("", p4kx_extraction::Entry::path)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use p4kx_testkit::{ArchiveBuilder, TestContainer, TestElement, TestEntry, test_key};

    use super::*;

    const CONFIG: &str = "687af3db-4c0d-4e8d-9261-102274f8b6ed";

    fn guid() -> Guid {
        CONFIG.parse().expect("a GUID")
    }

    fn placed(path: &str, entry: Option<usize>) -> Placed {
        Placed {
            config: guid(),
            path: path.to_owned(),
            entry,
        }
    }

    fn summary(nested: &[usize], placed_paths: &[&str]) -> Summary {
        Summary {
            nested: nested.to_vec(),
            placed: placed_paths.iter().map(|path| placed(path, None)).collect(),
            ..Summary::default()
        }
    }

    fn paths(found: &[&Placed]) -> Vec<String> {
        let mut paths: Vec<String> = found.iter().map(|placed| placed.path.clone()).collect();
        paths.sort_unstable();
        paths
    }

    #[test]
    fn below_collects_the_root_and_everything_it_places() {
        let summaries = HashMap::from([
            (0, summary(&[1, 2], &["root"])),
            (1, summary(&[3], &["one"])),
            (2, summary(&[3], &["two"])),
            (3, summary(&[], &["three"])),
            (9, summary(&[], &["elsewhere"])),
        ]);
        assert_eq!(
            paths(&below(0, &summaries)),
            ["one", "root", "three", "two"],
            "a container reached twice counts once"
        );
    }

    #[test]
    fn below_ends_a_cycle_where_it_returns() {
        let summaries = HashMap::from([(0, summary(&[1], &["a"])), (1, summary(&[0], &["b"]))]);
        assert_eq!(paths(&below(0, &summaries)), ["a", "b"]);
        assert_eq!(paths(&below(1, &summaries)), ["a", "b"]);
    }

    #[test]
    fn below_skips_a_container_that_was_not_read() {
        let summaries = HashMap::from([(0, summary(&[7], &["a"]))]);
        assert_eq!(paths(&below(0, &summaries)), ["a"]);
        assert_eq!(below(5, &summaries).len(), 0);
    }

    fn entity(container: &str, config: Option<&str>) -> String {
        let mut component = TestElement::new("EntityComponentObjectContainer")
            .attribute("objectContainer", container);
        if let Some(config) = config {
            component = component.attribute("multiConfigRef", config);
        }
        TestElement::new("Entity").child(component).to_xml()
    }

    fn socpak(name: &str, files: &[String]) -> TestEntry {
        let container =
            files
                .iter()
                .enumerate()
                .fold(TestContainer::new(), |container, (at, text)| {
                    container.entry(TestEntry::stored(
                        format!("entdata/{at}.entxml"),
                        text.clone(),
                    ))
                });
        TestEntry::stored(
            format!("Data\\ObjectContainers\\PU\\loc\\{name}.socpak"),
            container.build(),
        )
    }

    fn written(name: &str) -> String {
        format!("ObjectContainers/PU/loc/{name}.socpak")
    }

    fn open(dir: &Path, entries: Vec<TestEntry>) -> Archive {
        let path = dir.join("Data.p4k");
        entries
            .into_iter()
            .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
            .write_to(&path)
            .expect("a written Archive");
        Archive::open_with_key(path, Some(test_key())).expect("an opened Archive")
    }

    fn root_of(archive: &Archive, name: &str) -> Opened {
        let index = Index::new(archive);
        Opened {
            system: Arc::from("t"),
            entry: index.find(&written(name)).expect("a container"),
            under: None,
        }
    }

    #[test]
    fn a_cycle_of_containers_is_read_once_each() {
        let dir = tempfile::tempdir().expect("a directory");
        let archive = open(
            dir.path(),
            vec![
                socpak("a", &[entity(&written("b"), Some(CONFIG))]),
                socpak("b", &[entity(&written("a"), Some(CONFIG))]),
            ],
        );
        let index = Index::new(&archive);
        let root = root_of(&archive, "a");
        let explored = explore(&archive, &index, std::slice::from_ref(&root), None);
        assert_eq!(explored.summaries.len(), 2);
        assert_eq!(explored.beyond_limit, 0);
        let a = explored.summaries.get(&root.entry).expect("a");
        assert_eq!(a.placed.len(), 1);
        assert_eq!(a.placed[0].entry, index.find(&written("b")));
        assert_eq!(below(root.entry, &explored.summaries).len(), 2);
    }

    #[test]
    fn a_placed_container_the_archive_lacks_has_no_entry() {
        let dir = tempfile::tempdir().expect("a directory");
        let archive = open(
            dir.path(),
            vec![socpak("a", &[entity(&written("ghost"), Some(CONFIG))])],
        );
        let index = Index::new(&archive);
        let root = root_of(&archive, "a");
        let explored = explore(&archive, &index, std::slice::from_ref(&root), None);
        let a = explored.summaries.get(&root.entry).expect("a");
        assert_eq!(a.placed, [placed(&written("ghost"), None)]);
        assert_eq!(a.nested.len(), 0);
    }

    #[test]
    fn an_entity_file_with_several_links_is_noted() {
        let dir = tempfile::tempdir().expect("a directory");
        let two = TestElement::new("Entity")
            .child(TestElement::new("A").attribute("objectContainer", written("b")))
            .child(TestElement::new("B").attribute("objectContainer", written("c")))
            .to_xml();
        let archive = open(dir.path(), vec![socpak("a", &[two])]);
        let index = Index::new(&archive);
        let root = root_of(&archive, "a");
        let explored = explore(&archive, &index, std::slice::from_ref(&root), None);
        let a = explored.summaries.get(&root.entry).expect("a");
        assert_eq!(a.several, ["entdata\\0.entxml"]);
    }

    #[test]
    fn containers_beyond_the_limit_are_counted_not_read() {
        let dir = tempfile::tempdir().expect("a directory");
        let archive = open(
            dir.path(),
            vec![
                socpak(
                    "a",
                    &[entity(&written("b"), None), entity(&written("c"), None)],
                ),
                socpak("b", &[entity(&written("c"), None)]),
                socpak("c", &[]),
            ],
        );
        let index = Index::new(&archive);
        let root = root_of(&archive, "a");
        let explored = explore_within(&archive, &index, &[root], None, 2);
        assert_eq!(explored.summaries.len(), 2);
        assert_eq!(explored.beyond_limit, 1);
    }

    #[test]
    fn a_cancelled_walk_reads_nothing() {
        let dir = tempfile::tempdir().expect("a directory");
        let archive = open(dir.path(), vec![socpak("a", &[])]);
        let index = Index::new(&archive);
        let root = root_of(&archive, "a");
        let cancel = CancelToken::new();
        cancel.cancel();
        let explored = explore(&archive, &index, &[root], Some(&cancel));
        assert!(explored.summaries.is_empty());
    }
}
