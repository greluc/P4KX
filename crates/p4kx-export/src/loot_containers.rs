// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::path::Path;

use p4kx_extraction::{Archive, CancelToken, Guid};
use rayon::prelude::*;

use crate::contract_loot::{LootContainer, TagCount};
use crate::guid_text::CatalogGuid;
use crate::harvest_locations::Objects;
use crate::loot_entxml::{self, EntityLink};
use crate::loot_soc::{self, EntityTags};
use crate::shop_container_scan::{MAX_SOC_LEN, read_limited};
use crate::starmap_index::{Index, key};
use crate::starmap_walk::Opened;

/// The most distinct containers followed from the system containers.
const MAX_CONTAINERS: usize = 1 << 18;

/// The largest property file of one entity that is read, 1 MiB.
const MAX_ENTXML_LEN: u64 = 1 << 20;

/// The folder of a container that holds the property file of each entity.
const ENTITY_FOLDER: &str = "entdata/";

/// The extension of the property file of an entity.
const ENTITY_EXTENSION: &str = "entxml";

/// The loot containers found, and the containers that could not be read, each as its path and
/// the reason, ordered by path.
pub(crate) struct Located {
    pub(crate) containers: Vec<LootContainer>,
    pub(crate) unreadable: Vec<String>,
}

/// An entity of a container that carries a multi config.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placed {
    config: Guid,
    path: String,
    entry: Option<usize>,
}

/// What the entities of one container say: the containers they place and the configs they carry.
#[derive(Debug, Default)]
struct Summary {
    nested: Vec<usize>,
    placed: Vec<Placed>,
    unreadable: Option<&'static str>,
}

/// Returns the containers that carry a multi config in or below the containers `opened`, each
/// with the starmap object the root container hangs below. A container is followed through the
/// containers its entities place (`objectContainer`) and read once, in parallel; one placed below
/// several roots gives a row for each root. The rows are ordered by config, container, system and
/// object.
pub(crate) fn locate(
    archive: &Archive,
    opened: &[Opened],
    objects: &Objects,
    cancel: Option<&CancelToken>,
) -> Located {
    let index = Index::new(archive);
    let (summaries, unread) = explore(archive, &index, opened, cancel);
    let tags = read_tags(archive, &summaries, cancel);
    let mut containers = Vec::new();
    for root in opened {
        let location = objects.location(root.under);
        for placed in below(root.entry, &summaries) {
            containers.push(row(
                placed,
                &root.system,
                location,
                placed.entry.and_then(|entry| tags.get(&entry)),
            ));
        }
    }
    containers.sort_by_cached_key(|row| {
        (
            row.multi_config_guid.text(),
            row.container.clone(),
            row.system.clone(),
            row.starmap_object_guid.map(CatalogGuid::text),
        )
    });
    containers.dedup_by(|later, earlier| {
        later.multi_config_guid == earlier.multi_config_guid
            && later.container == earlier.container
            && later.system == earlier.system
            && later.starmap_object_guid == earlier.starmap_object_guid
    });
    let mut unreadable: Vec<String> = summaries
        .iter()
        .filter_map(|(&entry, summary)| {
            summary
                .unreadable
                .map(|reason| format!("{}: {reason}", path_of(archive, entry)))
        })
        .chain(tags.iter().filter_map(|(&entry, read)| {
            read.as_ref()
                .err()
                .map(|reason| format!("{}: {reason}", path_of(archive, entry)))
        }))
        .collect();
    unreadable.sort_unstable();
    if unread > 0 {
        unreadable.push(format!(
            "{unread} containers beyond the limit of {MAX_CONTAINERS} are not read"
        ));
    }
    Located {
        containers,
        unreadable,
    }
}

fn row(
    placed: &Placed,
    system: &str,
    location: Option<CatalogGuid>,
    tags: Option<&Result<EntityTags, &'static str>>,
) -> LootContainer {
    let counts = tags.and_then(|read| read.as_ref().ok());
    let mut tag_counts: Vec<TagCount> = counts
        .map(|counts| {
            counts
                .by_tag
                .iter()
                .map(|(&tag, &entities)| TagCount {
                    tag_guid: CatalogGuid(tag),
                    entities,
                })
                .collect()
        })
        .unwrap_or_default();
    tag_counts.sort_by_cached_key(|count| count.tag_guid.text());
    LootContainer {
        multi_config_guid: CatalogGuid(placed.config),
        container: placed.path.clone(),
        system: system.to_owned(),
        starmap_object_guid: location,
        entity_count: counts.map(|counts| counts.entities),
        tag_counts,
    }
}

/// Reads the containers in `opened` and every container their entities place, breadth first,
/// each once, until none is left or [`MAX_CONTAINERS`] are read; returns them and how many
/// containers the limit left unread.
fn explore(
    archive: &Archive,
    index: &Index<'_>,
    opened: &[Opened],
    cancel: Option<&CancelToken>,
) -> (HashMap<usize, Summary>, usize) {
    let mut summaries: HashMap<usize, Summary> = HashMap::new();
    let mut unread = 0;
    let mut frontier: Vec<usize> = opened.iter().map(|root| root.entry).collect();
    while !frontier.is_empty() && !cancel.is_some_and(CancelToken::is_cancelled) {
        frontier.sort_unstable();
        frontier.dedup();
        frontier.retain(|entry| !summaries.contains_key(entry));
        let room = MAX_CONTAINERS.saturating_sub(summaries.len());
        unread += frontier.len().saturating_sub(room);
        frontier.truncate(room);
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
    (summaries, unread)
}

fn summarise(archive: &Archive, index: &Index<'_>, entry: usize) -> Summary {
    let mut summary = Summary::default();
    let Some(socpak) = archive.entries().get(entry) else {
        summary.unreadable = Some("no such entry");
        return summary;
    };
    let Ok(container) = archive.open_container(socpak) else {
        summary.unreadable = Some("cannot be opened as a container");
        return summary;
    };
    for inner in container
        .entries()
        .iter()
        .filter(|inner| is_entity_file(inner.path()))
    {
        let Some(bytes) = read_limited(&container, inner, MAX_ENTXML_LEN) else {
            summary.unreadable = Some("an entity file cannot be read");
            continue;
        };
        let EntityLink {
            container: written,
            config,
        } = loot_entxml::read(&bytes);
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

/// Returns every [`Placed`] in `root` and the containers below it, each container once.
fn below(root: usize, summaries: &HashMap<usize, Summary>) -> Vec<&Placed> {
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

/// Reads the entity tags of the first `.soc` of every container a config is placed with, in
/// parallel.
fn read_tags(
    archive: &Archive,
    summaries: &HashMap<usize, Summary>,
    cancel: Option<&CancelToken>,
) -> HashMap<usize, Result<EntityTags, &'static str>> {
    let mut entries: Vec<usize> = summaries
        .values()
        .flat_map(|summary| summary.placed.iter().filter_map(|placed| placed.entry))
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries
        .par_iter()
        .filter(|_| !cancel.is_some_and(CancelToken::is_cancelled))
        .map(|&entry| (entry, soc_tags(archive, entry)))
        .collect()
}

fn soc_tags(archive: &Archive, entry: usize) -> Result<EntityTags, &'static str> {
    let socpak = archive.entries().get(entry).ok_or("no such entry")?;
    let container = archive
        .open_container(socpak)
        .map_err(|_| "cannot be opened as a container")?;
    let Some(soc) = container.entries().iter().find(|inner| {
        Path::new(inner.path())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("soc"))
    }) else {
        return Ok(EntityTags::default());
    };
    let bytes = read_limited(&container, soc, MAX_SOC_LEN).ok_or("its .soc is over the limit")?;
    loot_soc::tag_counts(&bytes)
}

fn path_of(archive: &Archive, entry: usize) -> &str {
    archive
        .entries()
        .get(entry)
        .map_or("", p4kx_extraction::Entry::path)
}
