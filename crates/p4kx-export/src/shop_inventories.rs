use std::collections::{BTreeSet, HashMap};
use std::io::Read;

use p4kx_extraction::{Archive, Entry};
use serde::Deserialize;

use crate::shop_registry::is_guid;
use crate::starmap_index::key;

const FOLDER: &str = "data\\scripts\\shopinventories\\";
const MAX_INVENTORY_LEN: u64 = 16 << 20;

/// What reading the inventory file of a shop gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ItemList {
    /// The item GUIDs of the file, lower case, ordered and without repeats.
    Items(Vec<String>),
    /// The Archive holds no such file.
    Missing,
    /// The file is too large, cannot be opened or is no inventory JSON.
    Unreadable,
}

/// The inventory files `Data\Scripts\ShopInventories\<stem>.json` of an Archive.
pub(crate) struct Inventories<'a> {
    archive: &'a Archive,
    by_stem: HashMap<String, &'a Entry>,
}

#[derive(Deserialize)]
struct File {
    #[serde(rename = "Collection")]
    collection: Option<Collection>,
    #[serde(rename = "Inventory")]
    inventory: Option<Vec<Element>>,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(rename = "Inventory")]
    inventory: Vec<Element>,
}

#[derive(Deserialize)]
struct Element {
    #[serde(rename = "ID")]
    id: Id,
}

#[derive(Deserialize)]
struct Id {
    #[serde(rename = "ID")]
    ids: Vec<String>,
}

impl<'a> Inventories<'a> {
    /// Indexes the `.json` Entries directly below `Data\Scripts\ShopInventories`.
    pub(crate) fn new(archive: &'a Archive) -> Self {
        let by_stem = archive
            .entries()
            .iter()
            .filter_map(|entry| {
                let path = key(entry.path());
                let file = path.strip_prefix(FOLDER)?;
                let stem = file.strip_suffix(".json")?;
                (!stem.is_empty() && !stem.contains('\\')).then(|| (stem.to_owned(), entry))
            })
            .collect();
        Self { archive, by_stem }
    }

    /// Reads the item GUIDs of the inventory file with the lower-case stem `stem`. Prices, stock
    /// and rental offerings are not read.
    pub(crate) fn items(&self, stem: &str) -> ItemList {
        let Some(entry) = self.by_stem.get(stem) else {
            return ItemList::Missing;
        };
        if entry.uncompressed_size() > MAX_INVENTORY_LEN {
            return ItemList::Unreadable;
        }
        let mut bytes = Vec::new();
        let read = self
            .archive
            .open_entry(entry)
            .map_err(drop)
            .and_then(|reader| {
                reader
                    .take(MAX_INVENTORY_LEN)
                    .read_to_end(&mut bytes)
                    .map_err(drop)
            });
        if read.is_err() {
            return ItemList::Unreadable;
        }
        parse(&bytes).map_or(ItemList::Unreadable, ItemList::Items)
    }
}

fn parse(bytes: &[u8]) -> Option<Vec<String>> {
    let file: File = serde_json::from_slice(bytes).ok()?;
    let elements = match (file.collection, file.inventory) {
        (Some(collection), _) => collection.inventory,
        (None, Some(inventory)) => inventory,
        (None, None) => return None,
    };
    let mut items = BTreeSet::new();
    for element in elements {
        for id in element.id.ids {
            if is_guid(&id) {
                items.insert(id.to_ascii_lowercase());
            }
        }
    }
    Some(items.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn only_the_item_ids_are_read_ordered_and_without_repeats() {
        let json = br#"{"ShopID":"x","Collection":{"Inventory":[
            {"ID":{"ID":["D195EB47-24B8-45C0-A4D0-1771D60DC1A8"]},"BuyPrice":1.5,"RentalOfferings":[]},
            {"ID":{"ID":["70a98c94-9252-4fd3-a673-5a584339e273"]},"BuyPrice":2.0},
            {"ID":{"ID":["d195eb47-24b8-45c0-a4d0-1771d60dc1a8"]}}]}}"#;
        assert_eq!(
            parse(json),
            Some(vec![
                "70a98c94-9252-4fd3-a673-5a584339e273".to_owned(),
                "d195eb47-24b8-45c0-a4d0-1771d60dc1a8".to_owned(),
            ])
        );
    }

    #[test]
    fn an_empty_inventory_is_an_empty_list_and_other_json_is_unreadable() {
        assert_eq!(
            parse(br#"{"Collection":{"Inventory":[]}}"#),
            Some(Vec::new())
        );
        assert_eq!(
            parse(br#"{"Inventory":[{"ID":{"ID":["70a98c94-9252-4fd3-a673-5a584339e273"]}}]}"#),
            Some(vec!["70a98c94-9252-4fd3-a673-5a584339e273".to_owned()]),
            "the shape without a Collection"
        );
        assert_eq!(parse(br#"{"Other":1}"#), None);
        assert_eq!(parse(b"not json"), None);
    }
}
