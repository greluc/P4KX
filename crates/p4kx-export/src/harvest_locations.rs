// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::path::Path;

use p4kx_extraction::{Archive, CancelToken, Guid};
use rayon::prelude::*;

use crate::contract::StarmapObject;
use crate::contract_harvest::HarvestableLocation;
use crate::guid_text::CatalogGuid;
use crate::soc_preset::PresetScan;
use crate::starmap_walk::Opened;

/// The most objects followed upwards from a container to find the object it belongs to.
const MAX_CHAIN: usize = 64;

/// The class-name start of the marker objects that sit beside a real location.
const PRIVATE_MINING_POINT: &str = "PrivateMiningPoint";

/// What the first `.soc` of a container says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Soc {
    Preset(Guid),
    Plain,
    Unreadable(&'static str),
}

/// The container locations found, and the containers that could not be read, each as its path and
/// the reason, ordered by path.
pub(crate) struct Located {
    pub(crate) locations: Vec<HarvestableLocation>,
    pub(crate) unreadable: Vec<String>,
}

/// The starmap objects a container can hang below, by GUID: the class name and the object above.
pub(crate) struct Objects<'a> {
    by_guid: HashMap<Guid, (&'a str, Option<Guid>)>,
}

impl<'a> Objects<'a> {
    pub(crate) fn new(objects: &'a [StarmapObject<'a>]) -> Self {
        Self {
            by_guid: objects
                .iter()
                .map(|object| {
                    (
                        object.guid.0,
                        (
                            object.class_name,
                            object.placed_under_guid.map(|guid| guid.0),
                        ),
                    )
                })
                .collect(),
        }
    }

    /// Returns the object a container hangs below: `under`, else the object above it while it is
    /// a private mining point; `None` for no object, or one the catalog lacks.
    fn location(&self, under: Option<Guid>) -> Option<CatalogGuid> {
        let mut at = under?;
        for _ in 0..MAX_CHAIN {
            let &(class_name, above) = self.by_guid.get(&at)?;
            if !is_private_mining_point(class_name) {
                return Some(CatalogGuid(at));
            }
            at = above?;
        }
        None
    }
}

fn is_private_mining_point(class_name: &str) -> bool {
    class_name
        .get(..PRIVATE_MINING_POINT.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(PRIVATE_MINING_POINT))
}

/// Returns the provider preset every container in `opened` names, for the presets in `presets`
/// only. Each container is read once, in parallel; a container hung below several objects gives
/// one location for each. The locations are ordered by preset, container, system and object.
pub(crate) fn locate(
    archive: &Archive,
    opened: &[Opened],
    presets: &HashSet<Guid>,
    objects: &Objects<'_>,
    cancel: Option<&CancelToken>,
) -> Located {
    let mut entries: Vec<usize> = opened.iter().map(|container| container.entry).collect();
    entries.sort_unstable();
    entries.dedup();
    let read: HashMap<usize, Soc> = entries
        .par_iter()
        .filter(|_| !cancel.is_some_and(CancelToken::is_cancelled))
        .map(|&entry| (entry, soc_of(archive, entry)))
        .collect();
    let mut unreadable: Vec<String> = read
        .iter()
        .filter_map(|(&entry, soc)| match soc {
            Soc::Unreadable(reason) => Some(format!("{}: {reason}", path_of(archive, entry))),
            _ => None,
        })
        .collect();
    unreadable.sort_unstable();
    let mut locations: Vec<HarvestableLocation> = opened
        .iter()
        .filter_map(|container| {
            let Some(Soc::Preset(preset)) = read.get(&container.entry) else {
                return None;
            };
            presets.contains(preset).then(|| HarvestableLocation {
                preset_guid: CatalogGuid(*preset),
                starmap_object_guid: objects.location(container.under),
                system: container.system.to_string(),
                container: path_of(archive, container.entry).to_owned(),
            })
        })
        .collect();
    locations.sort_by_cached_key(|location| {
        (
            location.preset_guid.text(),
            location.container.clone(),
            location.system.clone(),
            location.starmap_object_guid.map(CatalogGuid::text),
        )
    });
    locations.dedup_by(|later, earlier| {
        later.preset_guid == earlier.preset_guid
            && later.container == earlier.container
            && later.system == earlier.system
            && later.starmap_object_guid == earlier.starmap_object_guid
    });
    Located {
        locations,
        unreadable,
    }
}

fn path_of(archive: &Archive, entry: usize) -> &str {
    archive
        .entries()
        .get(entry)
        .map_or("", p4kx_extraction::Entry::path)
}

fn soc_of(archive: &Archive, entry: usize) -> Soc {
    let Some(socpak) = archive.entries().get(entry) else {
        return Soc::Unreadable("no such entry");
    };
    let Ok(container) = archive.open_container(socpak) else {
        return Soc::Unreadable("cannot be opened as a container");
    };
    let Some(soc) = container.entries().iter().find(|inner| {
        Path::new(inner.path())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("soc"))
    }) else {
        return Soc::Plain;
    };
    let Ok(reader) = container.open_entry(soc) else {
        return Soc::Unreadable("its .soc cannot be opened");
    };
    PresetScan::read(reader).map_or(Soc::Unreadable("its .soc cannot be read"), |preset| {
        preset.map_or(Soc::Plain, Soc::Preset)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_private_mining_point_is_left_for_the_object_above() {
        assert!(is_private_mining_point("PrivateMiningPoint_01"));
        assert!(is_private_mining_point("privateminingpoint"));
        assert!(!is_private_mining_point("Stanton1a"));
        assert!(!is_private_mining_point("Private"));
    }
}
