use std::collections::{HashMap, HashSet};

use crate::shop_registry::is_guid;

const MARK: &str = "EntityComponentShop";
const PROPERTY: &str = "offlineInventory";
const KNOWN: [&str; 10] = [
    "SCShop",
    MARK,
    "SShopComponentParams",
    "PropertiesDataCore",
    "__fullyInstanced",
    "__version",
    "__type",
    "CastShadowMinSpec",
    "AllowBakedRendering",
    "RenderProxy",
];
const MAX_PATH_LEN: usize = 256;
const MAX_BACK: usize = 12;

/// The identifiers a shop component can be tied to inside one container: the `CryGUID`s of its
/// `entitycomponentshop.xml` and the names of its `SCShop` objects, both of which the container
/// states for the same Entities.
#[derive(Debug, Default)]
pub(crate) struct Anchors {
    guids: HashSet<String>,
    names: HashMap<String, String>,
}

impl Anchors {
    /// Anchors for the lower-case `guids`, with the `names` of the objects that are unique in the
    /// container, each mapped to its lower-case `CryGUID`.
    pub(crate) fn new(guids: &[String], names: HashMap<String, String>) -> Self {
        let guids: HashSet<String> = guids.iter().cloned().collect();
        let names = names
            .into_iter()
            .filter(|(_, guid)| guids.contains(guid))
            .collect();
        Self { guids, names }
    }

    fn entity(&self, text: &str) -> Option<String> {
        if let Some(guid) = self.names.get(text) {
            return Some(guid.clone());
        }
        if is_guid(text) {
            let lower = text.to_ascii_lowercase();
            return self.guids.contains(&lower).then_some(lower);
        }
        None
    }
}

/// One shop component found in a container's `.soc`: the Entity it belongs to and the inventory
/// file its parameters name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShopComponent {
    pub(crate) entity: String,
    pub(crate) inventory: Option<String>,
}

/// Returns the shop components of the container file `bytes` that belong to an Entity of
/// `anchors`.
///
/// The file stores its strings once, in order of first use, so a later component repeats neither
/// the component name nor the property names. The inventory path is the value that follows an
/// `offlineInventory*` property name, or, where that name is not repeated, a path in a
/// `ShopInventories` folder; it belongs to the Entity whose `CryGUID` or object name is
/// the closest anchor before it, provided only the component name, the parameter type, property
/// names, positions, class names and at most one other GUID stand between them. A path with
/// anything else in between is dropped, so a component whose Entity cannot be told stays unknown.
/// A component name before which such an anchor stands and for which no path was found names no
/// inventory.
pub(crate) fn components(bytes: &[u8], anchors: &Anchors) -> Vec<ShopComponent> {
    if !contains(bytes, PROPERTY.as_bytes()) && !contains(bytes, MARK.as_bytes()) {
        return Vec::new();
    }
    let strings: Vec<&str> = bytes
        .split(|&byte| byte == 0)
        .map(|part| std::str::from_utf8(part).unwrap_or_default())
        .collect();
    let mut found: Vec<ShopComponent> = Vec::new();
    for (at, text) in strings.iter().enumerate() {
        if *text == MARK {
            if let Some(entity) = owner(&strings, at, anchors)
                && !found.iter().any(|component| component.entity == entity)
            {
                found.push(ShopComponent {
                    entity,
                    inventory: None,
                });
            }
        } else if let Some(stem) = inventory_stem(text)
            && (is_inventory_folder(text)
                || at
                    .checked_sub(1)
                    .and_then(|previous| strings.get(previous))
                    .is_some_and(|previous| is_property(previous)))
            && let Some(entity) = owner(&strings, at, anchors)
        {
            match found
                .iter_mut()
                .find(|component| component.entity == entity && component.inventory.is_none())
            {
                Some(component) => component.inventory = Some(stem),
                None => found.push(ShopComponent {
                    entity,
                    inventory: Some(stem),
                }),
            }
        }
    }
    found
}

fn owner(strings: &[&str], before: usize, anchors: &Anchors) -> Option<String> {
    let (mut foreign, mut marks) = (0_usize, 0_usize);
    for text in strings.get(..before)?.iter().rev().take(MAX_BACK) {
        if let Some(entity) = anchors.entity(text) {
            return Some(entity);
        }
        if *text == MARK {
            marks += 1;
            if marks > 1 {
                return None;
            }
        } else if is_guid(text) {
            foreign += 1;
            if foreign > 1 {
                return None;
            }
        } else if !(text.starts_with(PROPERTY) || KNOWN.contains(text) || is_position(text)) {
            return None;
        }
    }
    None
}

fn is_property(text: &str) -> bool {
    text.strip_prefix(PROPERTY)
        .is_some_and(|rest| rest.bytes().all(|byte| byte.is_ascii_alphabetic()))
}

fn is_inventory_folder(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("shopinventories/") || lower.contains("shopinventories\\")
}

fn is_position(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'+' | b'.' | b',' | b'e'))
}

/// Returns the lower-case file stem of an inventory file `value`, a path ending in `.json` or
/// `.xml`; `None` for anything else.
pub(crate) fn inventory_stem(value: &str) -> Option<String> {
    if value.len() > MAX_PATH_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return None;
    }
    let file = value.rsplit(['/', '\\']).next()?;
    let (stem, extension) = file.rsplit_once('.')?;
    let known = extension.eq_ignore_ascii_case("json") || extension.eq_ignore_ascii_case("xml");
    (known && !stem.is_empty()).then(|| stem.to_ascii_lowercase())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Anchors, components};

    const ENTITY: &str = "4dce4077-9730-ac77-5074-916b3b2700b7";
    const OTHER: &str = "4a000000-0000-4000-8000-0000000000aa";
    const CLASS: &str = "2845a83c-9c04-450f-99ce-43b8b7da6bec";

    fn anchors() -> Anchors {
        let names = HashMap::from([
            ("SCShop_First".to_owned(), ENTITY.to_owned()),
            ("SCShop_Second".to_owned(), OTHER.to_owned()),
        ]);
        Anchors::new(&[ENTITY.to_owned(), OTHER.to_owned()], names)
    }

    fn push(bytes: &mut Vec<u8>, parts: &[&str]) {
        for part in parts {
            bytes.extend_from_slice(part.as_bytes());
            bytes.push(0);
        }
    }

    fn stream(parts: &[&str]) -> Vec<u8> {
        let mut bytes = b"junk\0".to_vec();
        push(&mut bytes, parts);
        bytes
    }

    #[test]
    fn the_inventory_stem_is_lower_case_without_folder_and_extension() {
        for value in [
            "Data/scripts/shopinventories/Inv_Admin_GrimHex.json",
            "Scripts\\ShopInventories\\inv_admin_grimhex.xml",
        ] {
            let bytes = stream(&[
                ENTITY,
                "EntityComponentShop",
                "SShopComponentParams",
                "offlineInventoryJSON",
                value,
                "ShopRackGUIDs",
            ]);
            let found = components(&bytes, &anchors());
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].entity, ENTITY);
            assert_eq!(found[0].inventory.as_deref(), Some("inv_admin_grimhex"));
        }
    }

    #[test]
    fn an_empty_json_value_does_not_hide_the_xml_name_behind_it() {
        let bytes = stream(&[
            ENTITY,
            "EntityComponentShop",
            "SShopComponentParams",
            "offlineInventoryJSON",
            "offlineInventoryXMLName",
            "Scripts/ShopInventories/Inv_Late.xml",
        ]);
        assert_eq!(
            components(&bytes, &anchors())[0].inventory.as_deref(),
            Some("inv_late")
        );
    }

    #[test]
    fn a_component_without_a_path_names_no_inventory() {
        let bytes = stream(&[
            ENTITY,
            "EntityComponentShop",
            "SShopComponentParams",
            "offlineInventoryJSON",
            "eventSignalRadius",
            "shopInventoryType",
            "ITEM",
        ]);
        let found = components(&bytes, &anchors());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].inventory, None);
    }

    #[test]
    fn the_object_name_ties_a_component_whose_guid_string_is_not_stored_there() {
        let bytes = stream(&[
            "SCShop_First",
            "-12,592,444",
            "SCShop",
            CLASS,
            "EntityComponentShop",
            "SShopComponentParams",
            "offlineInventoryJSON",
            "scripts/shopinventories/inv_landing.json",
        ]);
        let found = components(&bytes, &anchors());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].entity, ENTITY);
        assert_eq!(found[0].inventory.as_deref(), Some("inv_landing"));
    }

    #[test]
    fn a_later_component_repeats_neither_component_name_nor_property_names() {
        let bytes = stream(&[
            ENTITY,
            "EntityComponentShop",
            "SShopComponentParams",
            "offlineInventoryJSON",
            "Data/inv_first.json",
            "eventSignalRadius",
            "ShopRackGUIDs",
            "SCShop_Second",
            "2.0,-5.4,1.5",
            OTHER,
            "scripts/shopinventories/inv_second.json",
        ]);
        let found = components(&bytes, &anchors());
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].inventory.as_deref(), Some("inv_first"));
        assert_eq!(found[1].entity, OTHER);
        assert_eq!(found[1].inventory.as_deref(), Some("inv_second"));
    }

    #[test]
    fn a_path_behind_foreign_strings_is_not_the_components() {
        let bytes = stream(&[
            ENTITY,
            "EntityComponentShop",
            "SShopComponentParams",
            "shopInventoryType",
            "ITEM",
            "ShopRackGUIDs",
            "offlineInventoryJSON",
            "Data/inv_other.json",
        ]);
        let found = components(&bytes, &anchors());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].inventory, None);
    }

    #[test]
    fn a_path_whose_entity_is_not_stored_stays_unknown() {
        let bytes = stream(&[
            "PropertiesDataCore",
            "__fullyInstanced",
            "__version",
            "2",
            "__type",
            "offlineInventoryJSON",
            "Data/inv_nobody.json",
        ]);
        assert_eq!(components(&bytes, &anchors()).len(), 0);
    }

    #[test]
    fn two_foreign_guids_between_anchor_and_path_break_the_tie() {
        let bytes = stream(&[
            ENTITY,
            CLASS,
            "4a000000-0000-4000-8000-0000000000bb",
            "offlineInventoryJSON",
            "Data/inv_far.json",
        ]);
        assert_eq!(components(&bytes, &anchors()).len(), 0);
    }

    #[test]
    fn text_without_a_component_gives_nothing() {
        assert_eq!(components(b"CrChF\0nothing", &anchors()).len(), 0);
        assert_eq!(components(b"EntityComponentShop", &anchors()).len(), 0);
    }
}
