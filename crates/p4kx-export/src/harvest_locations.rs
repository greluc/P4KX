// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
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

/// The starmap objects a container can hang below, by GUID: whether the object is a mining claim
/// marker and the object above it.
pub(crate) struct Objects {
    by_guid: HashMap<Guid, (bool, Option<Guid>)>,
}

impl Objects {
    pub(crate) fn new(objects: &[StarmapObject<'_>]) -> Self {
        Self {
            by_guid: objects
                .iter()
                .map(|object| {
                    (
                        object.guid.0,
                        (
                            object.mining_claim,
                            object.placed_under_guid.map(|guid| guid.0),
                        ),
                    )
                })
                .collect(),
        }
    }

    /// Returns the object a container hangs below: `under`, else the object above it while it is
    /// a mining claim marker; `None` for no object, or one the catalog lacks.
    pub(crate) fn location(&self, under: Option<Guid>) -> Option<CatalogGuid> {
        let mut at = under?;
        for _ in 0..MAX_CHAIN {
            let &(mining_claim, above) = self.by_guid.get(&at)?;
            if !mining_claim {
                return Some(CatalogGuid(at));
            }
            at = above?;
        }
        None
    }
}

/// Returns the provider preset every container in `opened` names, for the presets in `presets`
/// only. Each container is read once, in parallel; a container hung below several objects gives
/// one location for each. The locations are ordered by preset, container, system and object.
pub(crate) fn locate(
    archive: &Archive,
    opened: &[Opened],
    presets: &HashSet<Guid>,
    objects: &Objects,
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
    match PresetScan::read(reader) {
        Ok(preset) => preset.map_or(Soc::Plain, Soc::Preset),
        Err(error) if error.kind() == ErrorKind::FileTooLarge => {
            Soc::Unreadable("its .soc is over the scan limit")
        }
        Err(_) => Soc::Unreadable("its .soc cannot be read"),
    }
}
