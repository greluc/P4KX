// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::Guid;
use p4kx_extraction::cryxml::{self, Xml};

use crate::starmap_xml::{Node, attribute};

/// The attribute of an entity component that names the container the entity places.
const CONTAINER_ATTRIBUTE: &str = "objectContainer";

/// The attribute of a harvestable component that names its `SubHarvestableMultiConfigRecord`.
const CONFIG_ATTRIBUTE: &str = "multiConfigRef";

/// What the property file of one entity (`entdata/*.entxml` of a container) says about the
/// container it places and the multi config it carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EntityLink {
    /// The path of the container the entity places, as written.
    pub(crate) container: Option<String>,
    /// The `SubHarvestableMultiConfigRecord` the entity carries.
    pub(crate) config: Option<Guid>,
}

/// Reads the property file `bytes` of an entity, `CryXml` or text XML; the empty link for a file
/// that holds neither attribute or is no XML.
pub(crate) fn read(bytes: &[u8]) -> EntityLink {
    if !contains(bytes, CONTAINER_ATTRIBUTE) && !contains(bytes, CONFIG_ATTRIBUTE) {
        return EntityLink::default();
    }
    match cryxml::read(bytes) {
        Ok(Xml::CryXml(document)) => link_below(document.root()),
        Ok(Xml::Text(text)) => {
            let Ok(text) = std::str::from_utf8(text) else {
                return EntityLink::default();
            };
            let text = text.trim_start_matches('\u{feff}');
            if cryxml::check_text_depth(text).is_err() {
                return EntityLink::default();
            }
            roxmltree::Document::parse(text).map_or_else(
                |_| EntityLink::default(),
                |document| link_below(document.root_element()),
            )
        }
        Err(_) => EntityLink::default(),
    }
}

fn contains(bytes: &[u8], needle: &str) -> bool {
    bytes
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// Returns the first container and the first config named by an element at or below `root`.
fn link_below<'d, N: Node<'d>>(root: N) -> EntityLink {
    let mut link = EntityLink::default();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if link.container.is_none()
            && let Some(path) = attribute(node, CONTAINER_ATTRIBUTE)
            && !path.is_empty()
        {
            link.container = Some(path.to_owned());
        }
        if link.config.is_none()
            && let Some(guid) = attribute(node, CONFIG_ATTRIBUTE)
        {
            link.config = guid.trim().parse().ok();
        }
        pending.extend(node.children());
    }
    link
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "687af3db-4c0d-4e8d-9261-102274f8b6ed";

    fn guid(text: &str) -> Guid {
        text.parse().expect("a GUID")
    }

    #[test]
    fn a_harvestable_container_names_both() {
        let xml = format!(
            r#"<Entity Name="loot"><PropertiesDataCore>
            <EntityComponentObjectContainer objectContainer="ObjectContainers/PU/loot.socpak"/>
            <HarvestableComponent><looseSubConfigBase multiConfigRef="{CONFIG}"/></HarvestableComponent>
            </PropertiesDataCore></Entity>"#
        );
        assert_eq!(
            read(xml.as_bytes()),
            EntityLink {
                container: Some("ObjectContainers/PU/loot.socpak".to_owned()),
                config: Some(guid(CONFIG)),
            }
        );
    }

    #[test]
    fn a_plain_container_names_only_the_container() {
        let xml =
            r#"<Entity><EntityComponentObjectContainer objectContainer="a/b.socpak"/></Entity>"#;
        let link = read(xml.as_bytes());
        assert_eq!(link.container.as_deref(), Some("a/b.socpak"));
        assert_eq!(link.config, None);
    }

    #[test]
    fn a_file_with_neither_attribute_is_the_empty_link() {
        let empty = EntityLink::default();
        assert_eq!(read(b"<Entity Name=\"x\"/>"), empty);
        assert_eq!(read(b"not xml but objectContainer"), empty);
        assert_eq!(read(b""), empty);
    }

    #[test]
    fn a_config_that_is_no_guid_is_none() {
        let link = read(br#"<Entity><a multiConfigRef="nope"/></Entity>"#);
        assert_eq!(link, EntityLink::default());
    }
}
