// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, CancelToken, Entry};
use rayon::prelude::*;

use crate::shop_soc::inventory_stem;
use crate::starmap_index::key;
use crate::starmap_xml::{Node, attribute};

const FOLDER: &str = "data\\prefabs\\shops\\";
const MAX_PREFAB_LEN: u64 = 64 << 20;
const PROPERTY: &str = "offlineInventory";

/// The inventory files that the shop prefabs of an Archive name, by the `CryGUID` of the Entity.
#[derive(Debug, Default)]
pub(crate) struct Prefabs {
    pub(crate) by_entity: HashMap<String, Vec<String>>,
    pub(crate) listed: usize,
    pub(crate) unreadable: usize,
}

impl Prefabs {
    /// Reads every prefab library below `Data\Prefabs\Shops`. A shop Entity of a prefab has an
    /// `Id` that is its `CryGUID` with the bytes reordered, which the containers that instance
    /// the prefab carry in their `entitycomponentshop.xml`.
    pub(crate) fn read(archive: &Archive, cancel: Option<&CancelToken>) -> Self {
        let listed: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| {
                let path = key(entry.path());
                path.starts_with(FOLDER)
                    && Path::new(&path)
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
            })
            .collect();
        let read: Vec<Option<Vec<(String, String)>>> = listed
            .par_iter()
            .map(|entry| {
                if cancel.is_some_and(CancelToken::is_cancelled) {
                    return None;
                }
                named(archive, entry)
            })
            .collect();
        let mut prefabs = Self {
            listed: listed.len(),
            ..Self::default()
        };
        for pairs in read {
            let Some(pairs) = pairs else {
                prefabs.unreadable += 1;
                continue;
            };
            for (entity, stem) in pairs {
                let stems = prefabs.by_entity.entry(entity).or_default();
                if !stems.contains(&stem) {
                    stems.push(stem);
                }
            }
        }
        prefabs
    }
}

fn named(archive: &Archive, entry: &Entry) -> Option<Vec<(String, String)>> {
    if entry.uncompressed_size() > MAX_PREFAB_LEN {
        return None;
    }
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .ok()?
        .take(MAX_PREFAB_LEN)
        .read_to_end(&mut bytes)
        .ok()?;
    match cryxml::read(&bytes).ok()? {
        Xml::CryXml(document) => Some(pairs(document.root())),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text)
                .ok()?
                .trim_start_matches('\u{feff}');
            cryxml::check_text_depth(text).ok()?;
            let document = roxmltree::Document::parse(text).ok()?;
            Some(pairs(document.root_element()))
        }
    }
}

fn pairs<'d, N: Node<'d>>(root: N) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.name().eq_ignore_ascii_case("Object")
            && attribute(node, "Type") == Some("Entity")
            && let Some(entity) = attribute(node, "Id").and_then(crypt_guid)
        {
            for stem in shop_stems(node) {
                found.push((entity.clone(), stem));
            }
        }
        pending.extend(node.children());
    }
    found
}

fn shop_stems<'d, N: Node<'d>>(object: N) -> Vec<String> {
    object
        .children()
        .filter(|child| child.name().eq_ignore_ascii_case("PropertiesDataCore"))
        .flat_map(N::children)
        .filter(|child| child.name().eq_ignore_ascii_case("EntityComponentShop"))
        .flat_map(|component| component.attributes().collect::<Vec<_>>())
        .filter(|(name, _)| {
            name.get(..PROPERTY.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(PROPERTY))
        })
        .filter_map(|(_, value)| inventory_stem(value))
        .collect()
}

/// Returns the lower-case `CryGUID` of the editor `Id` `{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}`:
/// `CCCCBBBB-AAAA(first four)-AAAA(last four)-` and the bytes `DDDDEEEEEEEEEEEE` reversed.
fn crypt_guid(id: &str) -> Option<String> {
    let hex: String = id
        .trim_matches(['{', '}'])
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_ascii_lowercase();
    if hex.len() != 32 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (a, rest) = hex.split_at(8);
    let (b, rest) = rest.split_at(4);
    let (c, tail) = rest.split_at(4);
    let reversed: String = (0..tail.len() / 2)
        .rev()
        .map(|at| &tail[at * 2..at * 2 + 2])
        .collect();
    Some(format!(
        "{c}{b}-{}-{}-{}-{}",
        &a[..4],
        &a[4..],
        &reversed[..4],
        &reversed[4..]
    ))
}

#[cfg(test)]
mod tests {
    use super::crypt_guid;

    #[test]
    fn the_editor_id_becomes_the_container_guid() {
        assert_eq!(
            crypt_guid("{9730AC77-4077-4DCE-B700-273B6B917450}").as_deref(),
            Some("4dce4077-9730-ac77-5074-916b3b2700b7")
        );
        assert_eq!(crypt_guid("{9730AC77-4077}"), None);
        assert_eq!(crypt_guid("{ZZ30AC77-4077-4DCE-B700-273B6B917450}"), None);
    }
}
