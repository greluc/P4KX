use crate::shop_registry::is_guid;

const MARK: &[u8] = b"\0EntityComponentShop\0";
const PARAMS: &[u8] = b"SShopComponentParams\0";
const GUID_LEN: usize = 36;
const WINDOW: usize = 512;
const ENDS: [&[u8]; 4] = [
    b"ShopRackGUIDs",
    b"ShopKioskGUIDs",
    b"LandingAreaNodeGUIDs",
    MARK,
];
const PROPERTY: &[u8] = b"offlineInventory";
const MAX_PATH_LEN: usize = 256;

/// One `EntityComponentShop` of a container's `.soc`: the `SCShop` Entity and the inventory file
/// its parameters name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShopComponent {
    pub(crate) entity: String,
    pub(crate) inventory: Option<String>,
}

/// Returns the shop components of the container file `bytes`, in file order. The `CryGUID` of the
/// Entity stands right before the component name; the parameters, when the component has any,
/// follow `SShopComponentParams` as name and value strings up to the rack, kiosk or landing area
/// lists or the next component. `inventory` is the lower-case file stem of the first
/// `offlineInventory*` value that names a `.json` or `.xml` file, `None` when there is none.
pub(crate) fn components(bytes: &[u8]) -> Vec<ShopComponent> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(relative) = bytes.get(from..).and_then(|rest| find(rest, MARK)) {
        let at = from + relative;
        from = at + MARK.len();
        let Some(entity) = at
            .checked_sub(GUID_LEN)
            .and_then(|start| bytes.get(start..at))
            .and_then(|text| std::str::from_utf8(text).ok())
            .filter(|text| is_guid(text))
        else {
            continue;
        };
        let rest = bytes.get(from..).unwrap_or_default();
        let window = rest.get(..WINDOW.min(rest.len())).unwrap_or_default();
        let inventory = window.strip_prefix(PARAMS).and_then(|parameters| {
            let end = ENDS
                .iter()
                .filter_map(|end| find(parameters, end))
                .min()
                .unwrap_or(parameters.len());
            inventory_stem(parameters.get(..end)?)
        });
        found.push(ShopComponent {
            entity: entity.to_ascii_lowercase(),
            inventory,
        });
    }
    found
}

fn inventory_stem(window: &[u8]) -> Option<String> {
    let mut from = 0;
    while let Some(relative) = window.get(from..).and_then(|rest| find(rest, PROPERTY)) {
        from += relative + PROPERTY.len();
        if let Some(stem) = window.get(from..).and_then(value_stem) {
            return Some(stem);
        }
    }
    None
}

fn value_stem(rest: &[u8]) -> Option<String> {
    let name_end = rest.iter().position(|&byte| byte == 0)?;
    if !rest.get(..name_end)?.iter().all(u8::is_ascii_alphabetic) {
        return None;
    }
    let value = rest.get(name_end + 1..)?;
    let value = value.get(..value.iter().position(|&byte| byte == 0)?)?;
    if value.len() > MAX_PATH_LEN
        || !value
            .iter()
            .all(|&byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return None;
    }
    let path = std::str::from_utf8(value).ok()?;
    let file = path.rsplit(['/', '\\']).next()?;
    let (stem, extension) = file.rsplit_once('.')?;
    let known = extension.eq_ignore_ascii_case("json") || extension.eq_ignore_ascii_case("xml");
    (known && !stem.is_empty()).then(|| stem.to_ascii_lowercase())
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::components;

    const ENTITY: &str = "4dce4077-9730-ac77-5074-916b3b2700b7";
    const OTHER: &str = "4a000000-0000-4000-8000-0000000000aa";

    fn push(bytes: &mut Vec<u8>, parts: &[&str]) {
        for part in parts {
            bytes.extend_from_slice(part.as_bytes());
            bytes.push(0);
        }
    }

    fn component(entity: &str, parameters: &[&str], end: &str) -> Vec<u8> {
        let mut bytes = b"junk\0SCShop\0".to_vec();
        push(
            &mut bytes,
            &[entity, "EntityComponentShop", "SShopComponentParams"],
        );
        push(&mut bytes, parameters);
        push(&mut bytes, &[end, "tail"]);
        bytes
    }

    #[test]
    fn the_inventory_stem_is_lower_case_without_folder_and_extension() {
        for value in [
            "Data/scripts/shopinventories/Inv_Admin_GrimHex.json",
            "Scripts\\ShopInventories\\inv_admin_grimhex.xml",
        ] {
            let bytes = component(ENTITY, &["offlineInventoryJSON", value], "ShopRackGUIDs");
            let found = components(&bytes);
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].entity, ENTITY);
            assert_eq!(found[0].inventory.as_deref(), Some("inv_admin_grimhex"));
        }
    }

    #[test]
    fn a_component_without_a_value_names_no_inventory() {
        let bytes = component(
            ENTITY,
            &[
                "offlineInventoryJSON",
                "eventSignalRadius",
                "franchise",
                "x",
            ],
            "ShopKioskGUIDs",
        );
        assert_eq!(components(&bytes)[0].inventory, None);
        let plain = component(
            ENTITY,
            &["shopInventoryType", "ITEM"],
            "LandingAreaNodeGUIDs",
        );
        assert_eq!(components(&plain)[0].inventory, None);
    }

    #[test]
    fn an_empty_json_value_does_not_hide_the_xml_name_behind_it() {
        let bytes = component(
            ENTITY,
            &[
                "offlineInventoryJSON",
                "offlineInventoryXMLName",
                "Scripts/ShopInventories/Inv_Late.xml",
            ],
            "ShopRackGUIDs",
        );
        assert_eq!(components(&bytes)[0].inventory.as_deref(), Some("inv_late"));
    }

    #[test]
    fn a_value_behind_the_end_marker_is_not_the_components() {
        let mut bytes = component(ENTITY, &["shopInventoryType", "ITEM"], "ShopRackGUIDs");
        bytes.extend_from_slice(b"offlineInventoryJSON\0Data/inv_other.json\0");
        assert_eq!(components(&bytes)[0].inventory, None);
    }

    #[test]
    fn a_value_of_the_next_adjacent_component_is_not_the_previous_ones() {
        let mut bytes = b"junk\0".to_vec();
        push(
            &mut bytes,
            &[
                ENTITY,
                "EntityComponentShop",
                "SShopComponentParams",
                "shopInventoryType",
                "ITEM",
                OTHER,
                "EntityComponentShop",
                "SShopComponentParams",
                "offlineInventoryJSON",
                "Data/inv_second.json",
            ],
        );
        let found = components(&bytes);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].inventory, None);
        assert_eq!(found[1].entity, OTHER);
        assert_eq!(found[1].inventory.as_deref(), Some("inv_second"));
    }

    #[test]
    fn a_component_without_parameters_is_found_and_names_no_inventory() {
        let mut bytes = b"junk\0".to_vec();
        push(
            &mut bytes,
            &[ENTITY, "EntityComponentShop", "offlineInventoryJSON"],
        );
        push(&mut bytes, &["Data/inv_not_params.json"]);
        let found = components(&bytes);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].inventory, None);
    }

    #[test]
    fn text_before_the_marker_that_is_no_guid_is_skipped() {
        let bytes = component("not-a-guid", &[], "ShopRackGUIDs");
        assert_eq!(components(&bytes).len(), 0);
        assert_eq!(components(b"EntityComponentShop").len(), 0);
    }
}
