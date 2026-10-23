use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use p4kx_assembly::{ArchiveFiles, LoadoutFileError, LoadoutFiles};
use p4kx_extraction::{Archive, CancelToken};

use crate::collected::Collected;
use crate::contract::{Shop, StarmapObject};
use crate::crafting_warning::CraftingWarning;
use crate::guid_text::CatalogGuid;
use crate::shop_containers::{Containers, Found, Inventory};
use crate::shop_inventories::{Inventories, ItemList};
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
    let registry = match files.read(REGISTRY) {
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
    let inventories = Inventories::new(archive);
    let lists: Lists<'_> = containers
        .inventory_stems()
        .into_iter()
        .map(|stem| (stem, inventories.items(stem)))
        .collect();
    summary.soc_components = containers.components;
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
            (_, Some(_)) => summary.with_items += 1,
            (Inventory::Ambiguous, None) => summary.inventory_ambiguous += 1,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use p4kx_extraction::Guid;

    use super::{Links, RegistryShop, ShopSummary, list};
    use crate::contract::StarmapObject;
    use crate::guid_text::CatalogGuid;
    use crate::shop_containers::Containers;

    const ROOT: &str = "4a269932-183e-362d-aacc-188e7debbea9";
    const BODY: &str = "4ce1f681-b479-5269-0536-3d7f02fdccaf";

    fn object(
        id: u8,
        class_name: &'static str,
        entity_name: Option<&str>,
    ) -> StarmapObject<'static> {
        StarmapObject {
            guid: CatalogGuid(Guid::from_stored([id; 16])),
            class_name,
            record_path: "",
            name: crate::contract::LocalisedText::default(),
            r#type: None,
            type_guid: None,
            parent_guid: None,
            hide_in_starmap: None,
            system: None,
            entity_name: entity_name.map(str::to_owned),
            placed_under_guid: None,
            position: None,
        }
    }

    fn entry(location: &str, entity_id: &str) -> RegistryShop {
        RegistryShop {
            platform: "StantonSystem".to_owned(),
            id: format!(
                "{ROOT},{BODY},{entity_id},{}",
                entity_id.to_ascii_uppercase()
            ),
            super_name: format!("Root,Body,{location},SCShop_X"),
            default_value: String::new(),
            shop_name: String::new(),
        }
    }

    #[test]
    fn a_name_of_two_starmap_objects_links_to_none_and_is_counted() {
        let starmap = [
            object(1, "Spot", None),
            object(2, "Other", Some("spot")),
            object(3, "Unique", None),
        ];
        let links = Links::new(&starmap);
        let id = |n: u8| format!("4a000000-0000-4000-8000-0000000000{n:02x}");
        let registry = [
            entry("Spot", &id(1)),
            entry("Unique", &id(2)),
            entry("Nowhere", &id(3)),
        ];
        let mut summary = ShopSummary::default();
        let lists = HashMap::new();
        let shops = list(
            &registry,
            &Containers::default(),
            &links,
            &lists,
            &mut summary,
        );
        assert_eq!(shops.len(), 3);
        assert_eq!((summary.linked, summary.unlinked), (1, 2));
        assert_eq!(summary.ambiguous_link, 1, "only Spot is ambiguous");
        let spot = shops
            .iter()
            .find(|shop| shop.location.as_deref() == Some("Spot"));
        assert!(spot.is_some_and(|shop| shop.starmap_object_guid.is_none()));
    }

    #[test]
    fn a_key_is_lower_case_and_a_non_guid_element_is_malformed() {
        let links = Links::new(&[]);
        let upper = entry("Place", "4A000000-0000-4000-8000-0000000000AA");
        let lower = entry("Place", "4a000000-0000-4000-8000-0000000000aa");
        let broken = entry("Place", "not-a-guid");
        let registry = [upper, lower, broken];
        let mut summary = ShopSummary::default();
        let lists = HashMap::new();
        let shops = list(
            &registry,
            &Containers::default(),
            &links,
            &lists,
            &mut summary,
        );
        assert_eq!(shops.len(), 1, "the case-duplicate is dropped");
        assert_eq!((summary.duplicate, summary.malformed), (1, 1));
        let shop = &shops[0];
        assert_eq!(shop.system, "stanton");
        assert!(
            shop.key.ends_with(&shop.entity_guid),
            "entityGuid is the last element"
        );
        assert_eq!(shop.key, shop.key.to_ascii_lowercase());
    }
}
