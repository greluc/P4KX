use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use std::path::Path;

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, CancelToken, Entry};
use rayon::prelude::*;

use crate::CatalogError;
use crate::shop_soc;
use crate::starmap_index::key;
use crate::starmap_xml::{Node, attribute};

const SHOPS: &str = "data\\objectcontainers\\pu\\shops\\";
const LOC: &str = "data\\objectcontainers\\pu\\loc\\";
const LOC_KINDS: [&str; 2] = ["shopcomm", "shopfood"];
const SHOP_METADATA: &str = "\\metadata\\entitycomponentshop.xml";
const MAX_METADATA_LEN: u64 = 1 << 20;
const MAX_SOC_LEN: u64 = 64 << 20;

/// The kind and the brand of a shop container, from its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Class {
    pub(crate) kind: String,
    pub(crate) brand: Option<String>,
}

/// What the containers say of one `SCShop` Entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Found<'a> {
    /// No container names the Entity.
    Nowhere,
    /// Every container that names it has this kind and brand.
    One(&'a Class),
    /// The containers that name it differ in kind or brand.
    Mixed,
}

/// What the containers say of the inventory file of one `SCShop` Entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Inventory<'a> {
    /// No container names the Entity, or every container that does has the component and it
    /// names no inventory file.
    None,
    /// Every container that names the Entity names this file (lower-case stem).
    One(&'a str),
    /// The containers differ, or only some of them name a file.
    Ambiguous,
    /// No container that names the Entity has its component in a readable `.soc`, so nothing is
    /// known.
    Unknown,
}

/// What one container's `.soc` files say of the component of one `SCShop` Entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Component {
    /// No readable `.soc` of the container holds the component.
    NotFound,
    /// The component is there and names no inventory file.
    NoFile,
    /// The component names this file (lower-case stem).
    File(String),
    /// The container's components for the Entity disagree.
    Conflict,
}

type Shops = Vec<(String, Component)>;

#[derive(Debug, Default)]
struct Named {
    classes: Vec<Class>,
    inventories: Vec<Component>,
}

/// Combines what several containers or `.soc` files say of one Entity.
pub(crate) fn verdict(states: &[Component]) -> Inventory<'_> {
    let mut file: Option<&str> = None;
    let (mut conflict, mut not_found, mut no_file) = (false, false, false);
    for state in states {
        match state {
            Component::File(stem) => match file {
                Some(held) if held != stem => conflict = true,
                _ => file = Some(stem),
            },
            Component::Conflict => conflict = true,
            Component::NotFound => not_found = true,
            Component::NoFile => no_file = true,
        }
    }
    match file {
        _ if conflict => Inventory::Ambiguous,
        Some(_) if not_found || no_file => Inventory::Ambiguous,
        Some(stem) => Inventory::One(stem),
        None if not_found || states.is_empty() => Inventory::Unknown,
        None => Inventory::None,
    }
}

/// The `SCShop` Entities of the shop containers with the kind and brand of their path and the
/// inventory file their parameters name.
#[derive(Debug, Default)]
pub(crate) struct Containers {
    by_entity: HashMap<String, Named>,
    pub(crate) listed: usize,
    pub(crate) unreadable: usize,
    pub(crate) with_shop: usize,
    pub(crate) components: usize,
}

impl Containers {
    /// Opens every socpak below `PU\Shops` and the `shopcomm` and `shopfood` folders below
    /// `PU\loc` and reads the `CryGUID`s of its `entitycomponentshop.xml`.
    pub(crate) fn read(
        archive: &Archive,
        cancel: Option<&CancelToken>,
    ) -> Result<Self, CatalogError> {
        let listed: Vec<(&Entry, Class)> = archive
            .entries()
            .iter()
            .filter_map(|entry| classify(&key(entry.path())).map(|class| (entry, class)))
            .collect();
        let read: Vec<Option<Shops>> = listed
            .par_iter()
            .map(|(entry, _)| {
                if cancel.is_some_and(CancelToken::is_cancelled) {
                    return None;
                }
                shop_entities(archive, entry)
            })
            .collect();
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return Err(CatalogError::Cancelled);
        }
        let mut containers = Self {
            listed: listed.len(),
            ..Self::default()
        };
        for ((_, class), entities) in listed.into_iter().zip(read) {
            let Some(entities) = entities else {
                containers.unreadable += 1;
                continue;
            };
            if !entities.is_empty() {
                containers.with_shop += 1;
            }
            for (entity, inventory) in entities {
                containers.components += usize::from(inventory != Component::NotFound);
                let named = containers.by_entity.entry(entity).or_default();
                if !named.classes.contains(&class) {
                    named.classes.push(class.clone());
                }
                if !named.inventories.contains(&inventory) {
                    named.inventories.push(inventory);
                }
            }
        }
        Ok(containers)
    }

    /// Returns what the containers say of the Entity with the `CryGUID` `entity`, ignoring case.
    pub(crate) fn find(&self, entity: &str) -> Found<'_> {
        match self
            .by_entity
            .get(&entity.to_ascii_lowercase())
            .map(|named| named.classes.as_slice())
        {
            None | Some([]) => Found::Nowhere,
            Some([class]) => Found::One(class),
            Some(_) => Found::Mixed,
        }
    }

    /// Returns the inventory file the containers name for the Entity with the `CryGUID` `entity`,
    /// ignoring case.
    pub(crate) fn inventory(&self, entity: &str) -> Inventory<'_> {
        self.by_entity
            .get(&entity.to_ascii_lowercase())
            .map_or(Inventory::None, |named| verdict(&named.inventories))
    }

    /// Returns the inventory file stems any container names, ordered.
    pub(crate) fn inventory_stems(&self) -> BTreeSet<&str> {
        self.by_entity
            .values()
            .flat_map(|named| &named.inventories)
            .filter_map(|state| match state {
                Component::File(stem) => Some(stem.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Returns the kind and brand of the socpak with the normalised path `path`, `None` when it is no
/// shop container.
pub(crate) fn classify(path: &str) -> Option<Class> {
    if !path.ends_with(".socpak") {
        return None;
    }
    if let Some(rest) = path.strip_prefix(SHOPS) {
        let mut parts = rest.split('\\');
        let kind = parts
            .next()
            .filter(|kind| !kind.is_empty() && rest.contains('\\'))?;
        let brand = parts
            .next()
            .filter(|brand| !brand.is_empty() && parts.next().is_some());
        return Some(Class {
            kind: kind.to_owned(),
            brand: brand.map(str::to_owned),
        });
    }
    let rest = path.strip_prefix(LOC)?;
    let mut folders: Vec<&str> = rest.split('\\').collect();
    folders.pop();
    let kind = folders.last().copied()?;
    LOC_KINDS.contains(&kind).then(|| Class {
        kind: kind.to_owned(),
        brand: None,
    })
}

/// Returns the `CryGUID`s `entitycomponentshop.xml` of the container `socpak` names, lower case,
/// each with what the `.soc` files of the container say of its component; `None` when the
/// container or its `entitycomponentshop.xml` cannot be read. A `.soc` that cannot be read only
/// leaves the component `NotFound`: kind and brand do not depend on it.
fn shop_entities(archive: &Archive, socpak: &Entry) -> Option<Shops> {
    let container = archive.open_container(socpak).ok()?;
    let Some(metadata) = container.entries().iter().find(|inner| {
        let name = key(inner.path());
        name.ends_with(SHOP_METADATA) || name == SHOP_METADATA.trim_start_matches('\\')
    }) else {
        return Some(Vec::new());
    };
    let guids = entities(&read_limited(&container, metadata, MAX_METADATA_LEN)?)?;
    let mut found: HashMap<String, Vec<Component>> = HashMap::new();
    for inner in container.entries().iter().filter(|inner| {
        Path::new(inner.path())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("soc"))
    }) {
        let Some(bytes) = read_limited(&container, inner, MAX_SOC_LEN) else {
            continue;
        };
        for component in shop_soc::components(&bytes) {
            found.entry(component.entity).or_default().push(
                component
                    .inventory
                    .map_or(Component::NoFile, Component::File),
            );
        }
    }
    Some(
        guids
            .into_iter()
            .map(|guid| {
                let state = found
                    .get(&guid)
                    .map_or(Component::NotFound, |states| combined(states));
                (guid, state)
            })
            .collect(),
    )
}

fn combined(states: &[Component]) -> Component {
    match verdict(states) {
        Inventory::None => Component::NoFile,
        Inventory::One(stem) => Component::File(stem.to_owned()),
        Inventory::Ambiguous => Component::Conflict,
        Inventory::Unknown => Component::NotFound,
    }
}

fn read_limited(container: &Archive, entry: &Entry, limit: u64) -> Option<Vec<u8>> {
    if entry.uncompressed_size() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    container
        .open_entry(entry)
        .ok()?
        .take(limit)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

fn entities(bytes: &[u8]) -> Option<Vec<String>> {
    match cryxml::read(bytes).ok()? {
        Xml::CryXml(document) => Some(guids(document.root())),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text)
                .ok()?
                .trim_start_matches('\u{feff}');
            let document = roxmltree::Document::parse(text).ok()?;
            Some(guids(document.root_element()))
        }
    }
}

fn guids<'d, N: Node<'d>>(root: N) -> Vec<String> {
    root.children()
        .filter(|child| child.name().eq_ignore_ascii_case("Entity"))
        .filter_map(|entity| attribute(entity, "CryGUID"))
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Component, Inventory, classify, verdict};

    fn file(stem: &str) -> Component {
        Component::File(stem.to_owned())
    }

    #[test]
    fn containers_agree_only_when_all_name_the_same_file() {
        use Component::{Conflict, NoFile, NotFound};
        assert_eq!(verdict(&[]), Inventory::Unknown);
        assert_eq!(verdict(&[file("a")]), Inventory::One("a"));
        assert_eq!(verdict(&[file("a"), file("a")]), Inventory::One("a"));
        assert_eq!(verdict(&[file("a"), file("b")]), Inventory::Ambiguous);
        assert_eq!(verdict(&[NoFile, file("a")]), Inventory::Ambiguous);
        assert_eq!(verdict(&[file("a"), NoFile]), Inventory::Ambiguous);
        assert_eq!(verdict(&[file("a"), NotFound]), Inventory::Ambiguous);
        assert_eq!(verdict(&[Conflict]), Inventory::Ambiguous);
        assert_eq!(verdict(&[NoFile]), Inventory::None);
        assert_eq!(verdict(&[NoFile, NotFound]), Inventory::Unknown);
        assert_eq!(verdict(&[NotFound]), Inventory::Unknown);
    }

    #[test]
    fn kind_and_brand_come_from_the_folders() {
        let class = classify("data\\objectcontainers\\pu\\shops\\commex\\tdd\\tdd_newbab.socpak");
        assert_eq!(class.as_ref().map(|c| c.kind.as_str()), Some("commex"));
        assert_eq!(
            class.and_then(|c| c.brand),
            Some("tdd".to_owned()),
            "the second folder"
        );
        let bare = classify("data\\objectcontainers\\pu\\shops\\admin\\admin_portolisar.socpak");
        assert_eq!(bare.as_ref().map(|c| c.kind.as_str()), Some("admin"));
        assert_eq!(
            bare.and_then(|c| c.brand),
            None,
            "a file straight in the kind folder"
        );
        let module = classify(
            "data\\objectcontainers\\pu\\loc\\mod\\station\\util_a\\int\\module\\shopfood\\x.socpak",
        );
        assert_eq!(module.as_ref().map(|c| c.kind.as_str()), Some("shopfood"));
        assert_eq!(module.and_then(|c| c.brand), None);
    }

    #[test]
    fn other_containers_are_no_shop_containers() {
        for path in [
            "data\\objectcontainers\\pu\\loc\\mod\\station\\util_a\\int\\module\\other\\x.socpak",
            "data\\objectcontainers\\pu\\shops\\admin\\readme.txt",
            "data\\objectcontainers\\pu\\shops\\stray.socpak",
            "data\\objectcontainers\\ships\\x.socpak",
        ] {
            assert_eq!(classify(path), None, "{path}");
        }
    }
}
