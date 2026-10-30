// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_extraction::{Archive, Entry};

use crate::container_read::{MAX_SOC_LEN, read_limited, soc_entries};
use crate::shop_containers::{Component, Inventory, verdict};
use crate::shop_soc::{self, Anchors};
use crate::starmap_index::key;
use crate::starmap_xml::{Document, Node, attribute};

const SHOP_METADATA: &str = "\\metadata\\entitycomponentshop.xml";
const EDITOR_SUFFIX: &str = "_editor.xml";
const SHOP_TYPE: &str = "SCShop";
const MAX_METADATA_LEN: u64 = 1 << 20;
const MAX_EDITOR_LEN: u64 = 256 << 20;

/// The `CryGUID`s of a container with what its `.soc` files say of each one's component.
pub(crate) type Shops = Vec<(String, Component)>;

/// Returns the `CryGUID`s `entitycomponentshop.xml` of the container `socpak` names, lower case,
/// each with what the `.soc` files of the container say of its component; `None` when the
/// container or its `entitycomponentshop.xml` cannot be read, and how many inventory paths of its
/// `.soc` files no Entity could be told for. A `.soc` that cannot be read only
/// leaves the component `NotFound`: kind and brand do not depend on it.
pub(crate) fn shop_entities(archive: &Archive, socpak: &Entry) -> Option<(Shops, usize)> {
    let container = archive.open_container(socpak).ok()?;
    let Some(metadata) = container.entries().iter().find(|inner| {
        let name = key(inner.path());
        name.ends_with(SHOP_METADATA) || name == SHOP_METADATA.trim_start_matches('\\')
    }) else {
        return Some((Vec::new(), 0));
    };
    let guids = entities(&read_limited(&container, metadata, MAX_METADATA_LEN)?)?;
    let anchors = Anchors::new(&guids, object_names(&container).unwrap_or_default());
    let mut found: HashMap<String, Vec<Component>> = HashMap::new();
    let mut rejected = 0;
    for inner in soc_entries(&container) {
        let Some(bytes) = read_limited(&container, inner, MAX_SOC_LEN) else {
            continue;
        };
        let scan = shop_soc::components(&bytes, &anchors);
        rejected += scan.rejected;
        for component in scan.components {
            found.entry(component.entity).or_default().push(
                component
                    .inventory
                    .map_or(Component::NoFile, Component::File),
            );
        }
    }
    let shops = guids
        .into_iter()
        .map(|guid| {
            let state = found
                .get(&guid)
                .map_or(Component::NotFound, |states| combined(states));
            (guid, state)
        })
        .collect();
    Some((shops, rejected))
}

fn combined(states: &[Component]) -> Component {
    match verdict(states) {
        Inventory::None => Component::NoFile,
        Inventory::One(stem) => Component::File(stem.to_owned()),
        Inventory::Ambiguous => Component::Conflict,
        Inventory::Unknown => Component::NotFound,
    }
}

fn entities(bytes: &[u8]) -> Option<Vec<String>> {
    Some(guids(Document::parse(bytes).ok()?.root()))
}

fn guids<'d, N: Node<'d>>(root: N) -> Vec<String> {
    root.children()
        .filter(|child| child.name().eq_ignore_ascii_case("Entity"))
        .filter_map(|entity| attribute(entity, "CryGUID"))
        .map(str::to_ascii_lowercase)
        .collect()
}

fn object_names(container: &Archive) -> Option<HashMap<String, String>> {
    let editor = container.entries().iter().find(|inner| {
        let path = inner.path();
        !path.contains(['\\', '/']) && key(path).ends_with(EDITOR_SUFFIX)
    })?;
    let bytes = read_limited(container, editor, MAX_EDITOR_LEN)?;
    Some(shop_objects(Document::parse(&bytes).ok()?.root()))
}

fn shop_objects<'d, N: Node<'d>>(root: N) -> HashMap<String, String> {
    let mut named: HashMap<&str, Option<String>> = HashMap::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.name().eq_ignore_ascii_case("Object")
            && attribute(node, "type") == Some(SHOP_TYPE)
            && let (Some(name), Some(guid)) = (attribute(node, "name"), attribute(node, "guid"))
        {
            let guid = guid.to_ascii_lowercase();
            named
                .entry(name)
                .and_modify(|held| {
                    if held.as_ref() != Some(&guid) {
                        *held = None;
                    }
                })
                .or_insert(Some(guid));
        }
        pending.extend(node.children());
    }
    named
        .into_iter()
        .filter_map(|(name, guid)| guid.map(|guid| (name.to_owned(), guid)))
        .collect()
}

#[cfg(test)]
mod tests {
    use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

    use super::*;

    const METADATA: &str =
        r#"<Entities><Entity CryGUID="AB-1"/><Other/><Entity CryGUID="cd-2"/></Entities>"#;

    #[test]
    fn the_shop_entities_are_read_from_text_xml_with_or_without_a_byte_order_mark() {
        let expected = Some(vec!["ab-1".to_owned(), "cd-2".to_owned()]);
        assert_eq!(entities(METADATA.as_bytes()), expected);
        assert_eq!(entities(format!("\u{feff}{METADATA}").as_bytes()), expected);
    }

    #[test]
    fn the_shop_entities_are_read_from_cryxml() {
        let cry = TestElement::new("Entities")
            .child(TestElement::new("Entity").attribute("CryGUID", "AB-1"))
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
        assert_eq!(entities(&cry), Some(vec!["ab-1".to_owned()]));
    }

    #[test]
    fn bytes_that_are_no_xml_give_no_shop_entities() {
        assert_eq!(entities(b"not xml"), None);
        assert_eq!(entities(b"<a b=\"\xff\"/>"), None);
        assert_eq!(entities(b"<a><b></a>"), None);
    }

    #[test]
    fn a_shop_object_name_that_two_guids_claim_is_dropped_in_text_xml_with_a_mark() {
        let editor = "\u{feff}<Root><Object type=\"SCShop\" name=\"one\" guid=\"A\"/>\
            <Object type=\"SCShop\" name=\"two\" guid=\"B\"/>\
            <Object type=\"SCShop\" name=\"two\" guid=\"C\"/>\
            <Object type=\"Other\" name=\"three\" guid=\"D\"/></Root>";
        let document = Document::parse(editor.as_bytes()).expect("text XML");
        let named = shop_objects(document.root());
        assert_eq!(named.get("one").map(String::as_str), Some("a"));
        assert!(!named.contains_key("two"), "ambiguous");
        assert!(!named.contains_key("three"), "not a shop");
    }
}
