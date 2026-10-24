// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use p4kx_assembly::{ArchiveFiles, LoadoutFileError, LoadoutFiles};
use p4kx_extraction::{Archive, CancelToken};

use crate::collected::Collected;
use crate::contract::{Shop, StarmapObject};
use crate::crafting_warning::CraftingWarning;
use crate::shop_containers::{Containers, Found, Inventory};
use crate::shop_inventories::{Inventories, ItemList, is_placeholder};
use crate::shop_links::Links;
use crate::shop_prefabs::Prefabs;
use crate::shop_registry::{self, RegistryShop, clean, is_guid};
use crate::shop_summary::ShopSummary;
use crate::{CatalogError, CatalogOptions};

/// The shop registry of the layer recovery data.
pub(crate) const REGISTRY: &str = "Data\\Libs\\CosmoTools\\LayerRecovery\\LayerBackups.xml";

const SYSTEM_SUFFIX: &str = "system";
const COMMEX_PREFIX: &str = "scshop_commex_tdd_";
const PLATINUMBAY_PREFIX: &str = "scshop_platinumbay";
const COMMODITY_SUFFIX: &str = "_commodity";

/// The most unlinked location names the summary lists.
const LISTED_LOCATIONS: usize = 200;

/// The shops of the catalog and what reading them found.
pub(crate) struct Built {
    pub(crate) shops: Vec<Shop>,
    pub(crate) summary: ShopSummary,
}

/// Reads the shop registry and the shop containers into `collected` and returns the counts. With
/// a Filter no shop is read: shops are selected by no record path.
pub(crate) fn add_shops(
    archive: &Archive,
    files: &ArchiveFiles<'_>,
    collected: &mut Collected<'_>,
    options: &CatalogOptions<'_>,
) -> Result<ShopSummary, CatalogError> {
    if options.filter.is_some() {
        return Ok(ShopSummary::default());
    }
    let built = shops(archive, files, &collected.starmap_objects, options.cancel)?;
    let summary = built.summary;
    if summary.registry_unreadable {
        collected
            .warnings
            .push(CraftingWarning::UnreadableShopRegistry);
    } else if summary.registry_missing && !collected.starmap_objects.is_empty() {
        collected
            .warnings
            .push(CraftingWarning::MissingShopRegistry);
    }
    if summary.containers_unreadable > 0 {
        collected
            .warnings
            .push(CraftingWarning::UnreadableShopContainers(
                summary.containers_unreadable,
            ));
    }
    if summary.inventories_unreadable > 0 {
        collected
            .warnings
            .push(CraftingWarning::UnreadableShopInventories(
                summary.inventories_unreadable,
            ));
    }
    if summary.inventories_missing > 0 {
        collected
            .warnings
            .push(CraftingWarning::MissingShopInventories(
                summary.inventories_missing,
            ));
    }
    if summary.containers_with_shop > 0 && summary.soc_components == 0 {
        collected.warnings.push(CraftingWarning::NoShopComponents);
    }
    collected.shops = built.shops;
    Ok(summary)
}

/// Reads the shop registry of `archive` and the shop containers, and links each shop's location
/// to the starmap object of `starmap` that carries its name.
pub(crate) fn shops(
    archive: &Archive,
    files: &ArchiveFiles<'_>,
    starmap: &[StarmapObject<'_>],
    cancel: Option<&CancelToken>,
) -> Result<Built, CatalogError> {
    let mut summary = ShopSummary::default();
    let registry = read_registry(files, &mut summary);
    let Some(registry) = registry else {
        return Ok(Built {
            shops: Vec::new(),
            summary,
        });
    };
    if cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    let mut containers = Containers::read(archive, cancel)?;
    let prefabs = Prefabs::read(archive, cancel);
    if cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    containers.add_prefabs(&prefabs);
    summary.prefab_ambiguous = prefabs
        .by_entity
        .keys()
        .filter(|entity| matches!(containers.inventory(entity), Inventory::Ambiguous))
        .count();
    summary.count_sources(&containers, &prefabs);
    summary.registry_entries = registry.len();
    let links = Links::new(starmap);
    let inventories = Inventories::new(archive);
    let lists: Lists<'_> = containers
        .inventory_stems()
        .into_iter()
        .filter(|stem| !is_placeholder(stem))
        .map(|stem| (stem, inventories.items(stem)))
        .collect();
    summary.soc_components = containers.components;
    summary.inventories_named = containers.inventory_stems().len();
    summary.inventories_in_archive = inventories.count();
    summary.inventories_unreadable = lists
        .values()
        .filter(|list| **list == ItemList::Unreadable)
        .count();
    summary.inventories_missing = lists
        .values()
        .filter(|list| **list == ItemList::Missing)
        .count();
    summary.item_ids_dropped = lists
        .values()
        .map(|list| match list {
            ItemList::Items { dropped, .. } => *dropped,
            ItemList::Missing | ItemList::Unreadable => 0,
        })
        .sum();
    let shops = list(&registry, &containers, &links, &lists, &mut summary);
    Ok(Built { shops, summary })
}

fn read_registry(files: &ArchiveFiles<'_>, summary: &mut ShopSummary) -> Option<Vec<RegistryShop>> {
    match files.read(REGISTRY) {
        Ok(bytes) => {
            let parsed = shop_registry::read(&bytes);
            summary.registry_unreadable = parsed.is_none();
            parsed
        }
        Err(LoadoutFileError::NotFound) => {
            summary.registry_missing = true;
            None
        }
        Err(_) => {
            summary.registry_unreadable = true;
            None
        }
    }
}

type Lists<'a> = HashMap<&'a str, ItemList>;

/// Turns the registry entries into shops, ordered by key, and counts them in `summary`.
fn list(
    registry: &[RegistryShop],
    containers: &Containers,
    links: &Links,
    lists: &Lists<'_>,
    summary: &mut ShopSummary,
) -> Vec<Shop> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut built = Vec::with_capacity(registry.len());
    let mut unlinked: BTreeSet<String> = BTreeSet::new();
    let mut systems: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for raw in registry {
        let Some(shop) = shop(raw, containers, links, lists) else {
            summary.malformed += 1;
            continue;
        };
        if !seen.insert(shop.key.clone()) {
            summary.duplicate += 1;
            continue;
        }
        match (&shop.starmap_object_guid, &shop.location) {
            (Some(_), _) => summary.linked += 1,
            (None, location) => {
                summary.unlinked += 1;
                if let Some(location) = location {
                    if links.is_ambiguous(location) {
                        summary.ambiguous_link += 1;
                    }
                    unlinked.insert(location.clone());
                }
            }
        }
        match containers.find(&shop.entity_guid) {
            Found::One(_) => summary.with_kind += 1,
            Found::Mixed => summary.mixed_kind += 1,
            Found::Nowhere => summary.without_container += 1,
        }
        match (containers.inventory(&shop.entity_guid), &shop.item_guids) {
            (_, Some(_)) => {
                summary.with_items += 1;
                summary.prefab_only_shops += usize::from(containers.only_prefab(&shop.entity_guid));
            }
            (Inventory::Ambiguous, None) => summary.inventory_ambiguous += 1,
            (Inventory::One(stem), None) if is_placeholder(stem) => {
                summary.inventory_placeholder += 1;
            }
            (Inventory::One(stem), None) => match lists.get(stem) {
                Some(ItemList::Unreadable) => summary.inventory_unreadable += 1,
                _ => summary.inventory_missing += 1,
            },
            (Inventory::Unknown, None) => summary.inventory_unscanned += 1,
            (Inventory::None, None) => {}
        }
        summary.commodity_exchanges += usize::from(shop.commodity_exchange);
        *systems.entry(shop.system.clone()).or_default() += 1;
        if let Some(kind) = &shop.kind {
            *kinds.entry(kind.clone()).or_default() += 1;
        }
        built.push(shop);
    }
    built.sort_by(|a, b| a.key.cmp(&b.key));
    summary.shops = built.len();
    summary.systems = systems.into_iter().collect();
    summary.kinds = kinds.into_iter().collect();
    summary.unlinked_locations = unlinked.into_iter().take(LISTED_LOCATIONS).collect();
    built
}

fn shop(
    raw: &RegistryShop,
    containers: &Containers,
    links: &Links,
    lists: &Lists<'_>,
) -> Option<Shop> {
    let ids: Vec<String> = raw
        .id
        .split(',')
        .map(|id| id.trim().to_ascii_lowercase())
        .collect();
    let names: Vec<&str> = raw.super_name.split(',').collect();
    if ids.len() != names.len() || ids.len() < 2 || ids.iter().any(|id| !is_guid(id)) {
        return None;
    }
    let (entity, parents) = names.split_last()?;
    let entity_name = clean(entity).to_owned();
    let path: Vec<String> = parents.iter().map(|name| clean(name).to_owned()).collect();
    if entity_name.is_empty() || path.iter().any(String::is_empty) {
        return None;
    }
    let entity_guid = ids.last()?.clone();
    let location = path.get(2).cloned();
    let starmap_object_guid = location.as_deref().and_then(|name| links.find(name));
    let class = match containers.find(&entity_guid) {
        Found::One(class) => Some(class),
        Found::Nowhere | Found::Mixed => None,
    };
    let lower = entity_name.to_ascii_lowercase();
    let commodity_exchange = lower.starts_with(COMMEX_PREFIX)
        || (lower.starts_with(PLATINUMBAY_PREFIX) && lower.ends_with(COMMODITY_SUFFIX));
    let (inventory, item_guids) = match containers.inventory(&entity_guid) {
        Inventory::One(stem) => match lists.get(stem) {
            Some(ItemList::Items { guids, .. }) => (Some(stem.to_owned()), Some(guids.clone())),
            _ => (None, None),
        },
        Inventory::None | Inventory::Ambiguous | Inventory::Unknown => (None, None),
    };
    let non_empty = |text: &str| (!text.trim().is_empty()).then(|| text.trim().to_owned());
    let platform = raw.platform.to_ascii_lowercase();
    Some(Shop {
        key: ids.join(","),
        entity_guid,
        system: platform
            .strip_suffix(SYSTEM_SUFFIX)
            .unwrap_or(&platform)
            .to_owned(),
        entity_name,
        registry_name: non_empty(&raw.shop_name),
        default_value: non_empty(&raw.default_value),
        body: path.get(1).cloned(),
        location,
        containers: path,
        starmap_object_guid,
        kind: class.map(|class| class.kind.clone()),
        brand: class.and_then(|class| class.brand.clone()),
        commodity_exchange,
        inventory,
        item_guids,
    })
}

#[cfg(test)]
mod tests;
