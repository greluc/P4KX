use p4kx_extraction::cryxml::{self, Xml};

use crate::starmap_xml::{Node, attribute};

/// One `ShopBackup` entry of the shop registry, as it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryShop {
    pub(crate) platform: String,
    pub(crate) id: String,
    pub(crate) super_name: String,
    pub(crate) default_value: String,
    pub(crate) shop_name: String,
}

/// Reads the `ShopBackup` entries of the registry `bytes`, text XML or `CryXml`, in document order;
/// `None` when the bytes are neither.
pub(crate) fn read(bytes: &[u8]) -> Option<Vec<RegistryShop>> {
    match cryxml::read(bytes).ok()? {
        Xml::CryXml(document) => Some(collect(document.root())),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text)
                .ok()?
                .trim_start_matches('\u{feff}');
            let document = roxmltree::Document::parse(text).ok()?;
            Some(collect(document.root_element()))
        }
    }
}

fn named<'d, N: Node<'d>>(node: N, name: &'static str) -> impl Iterator<Item = N> {
    node.children()
        .filter(move |child| child.name().eq_ignore_ascii_case(name))
}

fn collect<'d, N: Node<'d>>(root: N) -> Vec<RegistryShop> {
    let mut shops = Vec::new();
    for backups in named(root, "LayerBackups") {
        for layer in named(backups, "LayerBackup") {
            let platform = attribute(layer, "PlatformName").unwrap_or_default();
            for list in named(layer, "ShopBackups") {
                for shop in named(list, "ShopBackup") {
                    let text = |name| attribute(shop, name).unwrap_or_default().to_owned();
                    shops.push(RegistryShop {
                        platform: platform.to_owned(),
                        id: text("ID"),
                        super_name: text("SuperName"),
                        default_value: text("DefaultValue"),
                        shop_name: text("ShopName"),
                    });
                }
            }
        }
    }
    shops
}

/// Returns `name` without a trailing `_{GUID}` (or `_{GUID}_socpak`) and without surrounding
/// blanks.
pub(crate) fn clean(name: &str) -> &str {
    let name = name.trim();
    let base = name.strip_suffix("_socpak").unwrap_or(name);
    match base
        .strip_suffix('}')
        .and_then(|rest| rest.rsplit_once("_{"))
    {
        Some((head, _)) => head.trim_end(),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn a_container_name_loses_its_guid_and_its_blanks() {
        assert_eq!(
            clean("OOC_Stanton_4_Microtech_{B4795269-F681-4CE1-AFCC-FD027F3D3605}"),
            "OOC_Stanton_4_Microtech"
        );
        assert_eq!(
            clean("Drlct_Stlmnt_008_Megaton_RastarLE "),
            "Drlct_Stlmnt_008_Megaton_RastarLE"
        );
        assert_eq!(
            clean("SCShop_Bar_Drlct_Stanton"),
            "SCShop_Bar_Drlct_Stanton"
        );
        assert_eq!(clean("odd_{not a guid"), "odd_{not a guid");
        assert_eq!(
            clean("pbay_cluster_001_sand_{04E2D380-091E-449D-8B48-08CB81FBCC08}_socpak"),
            "pbay_cluster_001_sand"
        );
        assert_eq!(clean("plain_socpak"), "plain_socpak");
    }
}
