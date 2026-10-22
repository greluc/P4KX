use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use p4kx_assembly::{ArchiveFiles, LoadoutFileError, LoadoutFiles};
use p4kx_extraction::{Archive, CancelToken};

use crate::CatalogError;
use crate::contract::{Shop, StarmapObject};
use crate::guid_text::CatalogGuid;
use crate::shop_containers::{Containers, Found};
use crate::shop_registry::{self, RegistryShop, clean};
use crate::summary::ShopSummary;

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

/// Reads the shop registry of `archive` and the shop containers, and links each shop's location
/// to the starmap object of `starmap` that carries its name.
pub(crate) fn shops(
    archive: &Archive,
    files: &ArchiveFiles<'_>,
    starmap: &[StarmapObject<'_>],
    cancel: Option<&CancelToken>,
) -> Result<Built, CatalogError> {
    let mut summary = ShopSummary::default();
    let registry = match files.read(REGISTRY) {
        Ok(bytes) => {
            let parsed = shop_registry::read(&bytes);
            summary.registry_unreadable = parsed.is_none();
            parsed
        }
        Err(LoadoutFileError::NotFound) => None,
        Err(_) => {
            summary.registry_unreadable = true;
            None
        }
    };
    let Some(registry) = registry else {
        return Ok(Built {
            shops: Vec::new(),
            summary,
        });
    };
    if cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    let containers = Containers::read(archive, cancel)?;
    summary.containers_listed = containers.listed;
    summary.containers_unreadable = containers.unreadable;
    summary.containers_with_shop = containers.with_shop;
    summary.registry_entries = registry.len();
    let links = Links::new(starmap);
    let shops = list(&registry, &containers, &links, &mut summary);
    Ok(Built { shops, summary })
}

/// Turns the registry entries into shops, ordered by key, and counts them in `summary`.
fn list(
    registry: &[RegistryShop],
    containers: &Containers,
    links: &Links,
    summary: &mut ShopSummary,
) -> Vec<Shop> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut built = Vec::with_capacity(registry.len());
    let mut unlinked: BTreeSet<String> = BTreeSet::new();
    let mut systems: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for raw in registry {
        let Some(shop) = shop(raw, containers, links) else {
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

fn shop(raw: &RegistryShop, containers: &Containers, links: &Links) -> Option<Shop> {
    let ids: Vec<&str> = raw.id.split(',').map(str::trim).collect();
    let names: Vec<&str> = raw.super_name.split(',').collect();
    if ids.len() != names.len() || ids.len() < 2 || ids.iter().any(|id| id.is_empty()) {
        return None;
    }
    let (entity, parents) = names.split_last()?;
    let entity_name = clean(entity).to_owned();
    if entity_name.is_empty() {
        return None;
    }
    let entity_guid = ids.last()?.to_ascii_lowercase();
    let path: Vec<String> = parents.iter().map(|name| clean(name).to_owned()).collect();
    let location = path.get(2).cloned();
    let starmap_object_guid = location.as_deref().and_then(|name| links.find(name));
    let class = match containers.find(&entity_guid) {
        Found::One(class) => Some(class),
        Found::Nowhere | Found::Mixed => None,
    };
    let lower = entity_name.to_ascii_lowercase();
    let commodity_exchange = lower.starts_with(COMMEX_PREFIX)
        || (lower.starts_with(PLATINUMBAY_PREFIX) && lower.ends_with(COMMODITY_SUFFIX));
    let non_empty = |text: &str| (!text.trim().is_empty()).then(|| text.trim().to_owned());
    Some(Shop {
        key: ids.join(","),
        entity_guid,
        system: raw
            .platform
            .strip_suffix(SYSTEM_SUFFIX)
            .unwrap_or(&raw.platform)
            .to_ascii_lowercase(),
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
    })
}

/// The starmap objects by the lower-case names that place them.
struct Links {
    by_name: HashMap<String, Option<CatalogGuid>>,
}

impl Links {
    fn new(starmap: &[StarmapObject<'_>]) -> Self {
        let mut by_name: HashMap<String, Option<CatalogGuid>> = HashMap::new();
        for object in starmap {
            let names = [Some(object.class_name), object.entity_name.as_deref()];
            for name in names.into_iter().flatten().filter(|name| !name.is_empty()) {
                by_name
                    .entry(name.to_ascii_lowercase())
                    .and_modify(|held| {
                        if *held != Some(object.guid) {
                            *held = None;
                        }
                    })
                    .or_insert(Some(object.guid));
            }
        }
        Self { by_name }
    }

    fn find(&self, name: &str) -> Option<CatalogGuid> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .copied()
            .flatten()
    }

    fn is_ambiguous(&self, name: &str) -> bool {
        matches!(self.by_name.get(&name.to_ascii_lowercase()), Some(None))
    }
}
