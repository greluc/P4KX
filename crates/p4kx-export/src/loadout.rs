use std::collections::{BTreeSet, HashSet};

use p4kx_assembly::{ArchiveFiles, LoadoutResolver};
use p4kx_extraction::{Archive, CancelToken, DataForge, Guid, RecordTree, Value};
use rayon::prelude::*;

use crate::contract_loadout::{Loadout, LoadoutPort};
use crate::guid_text::CatalogGuid;
use crate::loadout_port::ports;
use crate::record_fields::elements;
use crate::{CatalogError, CatalogFailure, LoadoutSummary};

const DEFAULT_LOADOUT: &str = "SEntityComponentDefaultLoadoutParams";
const PORT_CONTAINER: &str = "SItemPortContainerComponentParams";

/// The `entityKind` of a ship.
pub(crate) const SHIP: &str = "ship";
/// The `entityKind` of an item.
pub(crate) const ITEM: &str = "item";

/// Returns whether the Record tree `tree` may have ports: its Record defines item ports or names a
/// default Loadout. Only such an item is resolved.
pub(crate) fn carries_ports(tree: &RecordTree<'_>) -> bool {
    let Some(Value::Array(components)) = tree.field("Components") else {
        return false;
    };
    components.iter().any(|component| match component {
        Value::Struct(component) => match component.type_name() {
            DEFAULT_LOADOUT => true,
            PORT_CONTAINER => !elements(Some(component), "Ports").is_empty(),
            _ => false,
        },
        _ => false,
    })
}

/// An entry of `ships` or `items` whose ports are wanted.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Wanted {
    pub(crate) guid: CatalogGuid,
    pub(crate) kind: &'static str,
}

/// The loadouts of a catalog, the Records that could not be resolved and what was counted.
pub(crate) struct Loadouts {
    pub(crate) entries: Vec<Loadout>,
    pub(crate) failures: Vec<CatalogFailure>,
    pub(crate) summary: LoadoutSummary,
}

/// Resolves the Default Loadout of every entity in `wanted` and returns the ports of each,
/// ordered by GUID.
///
/// A ship always gets a loadout; an item gets one only when its tree holds a port. The tree stops
/// at docked vehicles (`LoadoutResolver::stopping_at_docked_vehicles`). `known` holds the GUIDs of
/// the entries of `items` and `ships`; the attached GUIDs outside it are counted in the summary.
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub(crate) fn loadouts<'f>(
    archive: &Archive,
    forge: &'f DataForge<'f>,
    wanted: &[Wanted],
    known: &HashSet<Guid>,
    cancel: Option<&CancelToken>,
) -> Result<Loadouts, CatalogError> {
    let files = ArchiveFiles::new(archive);
    let read: Vec<Option<Result<Option<Loadout>, CatalogFailure>>> = wanted
        .par_iter()
        .map_init(
            || {
                LoadoutResolver::new(forge)
                    .with_files(&files)
                    .stopping_at_docked_vehicles()
            },
            |resolver, want| {
                if cancel.is_some_and(CancelToken::is_cancelled) {
                    return None;
                }
                Some(read(resolver, forge, *want))
            },
        )
        .collect();
    let mut entries = Vec::with_capacity(read.len());
    let mut failures = Vec::new();
    for result in read {
        match result.ok_or(CatalogError::Cancelled)? {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}
            Err(failure) => failures.push(failure),
        }
    }
    entries.sort_by_cached_key(|entry| entry.entity_guid.text());
    let summary = LoadoutSummary::of(&entries, known);
    Ok(Loadouts {
        entries,
        failures,
        summary,
    })
}

fn read<'f>(
    resolver: &mut LoadoutResolver<'f>,
    forge: &'f DataForge<'f>,
    want: Wanted,
) -> Result<Option<Loadout>, CatalogFailure> {
    let Some(record) = forge.record_by_guid(want.guid.0) else {
        return Ok(None);
    };
    let node = resolver
        .resolve(&record)
        .map_err(|error| CatalogFailure::of(&record, &error))?;
    let mut issues = BTreeSet::new();
    let ports = ports(&node, &mut issues);
    if want.kind == ITEM && ports.is_empty() {
        return Ok(None);
    }
    Ok(Some(Loadout {
        entity_guid: want.guid,
        entity_kind: want.kind,
        issues: issues.into_iter().collect(),
        ports,
    }))
}

impl LoadoutSummary {
    /// Counts the ports of `entries` and the attached GUIDs outside `known`.
    fn of(entries: &[Loadout], known: &HashSet<Guid>) -> Self {
        let mut summary = Self {
            ships: entries
                .iter()
                .filter(|entry| entry.entity_kind == SHIP)
                .count(),
            items: entries
                .iter()
                .filter(|entry| entry.entity_kind == ITEM)
                .count(),
            with_issues: entries
                .iter()
                .filter(|entry| !entry.issues.is_empty())
                .count(),
            ..Self::default()
        };
        let mut outside = HashSet::new();
        for entry in entries {
            summary.count(&entry.ports, known, &mut outside);
        }
        summary.outside = outside.len();
        summary
    }

    fn count(&mut self, ports: &[LoadoutPort], known: &HashSet<Guid>, outside: &mut HashSet<Guid>) {
        for port in ports {
            self.ports = self.ports.saturating_add(1);
            if port.unresolved.is_some() {
                self.unresolved = self.unresolved.saturating_add(1);
            } else if let Some(guid) = port.item_guid {
                self.attached = self.attached.saturating_add(1);
                if !known.contains(&guid.0) {
                    outside.insert(guid.0);
                }
            }
            if port.docked_vehicle {
                self.docked_vehicles = self.docked_vehicles.saturating_add(1);
            }
            self.count(&port.ports, known, outside);
        }
    }
}
