// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic Archive whose shop containers name inventory files in their `.soc`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{TestContainer, TestElement, TestEntry};

use crate::shops_fixture::{CLOTHING, EXCHANGE, FOOD, LOOSE, MIXED, REGISTRY, registry};
use crate::starmap_fixture;

const SHOPS: &str = "Data\\ObjectContainers\\PU\\Shops\\";
const MODULE: &str = "Data\\ObjectContainers\\PU\\loc\\mod\\station\\util_a\\int\\module\\";
const INVENTORIES: &str = "Data\\Scripts\\ShopInventories\\";

pub(crate) const ITEM_A: &str = "d195eb47-24b8-45c0-a4d0-1771d60dc1a8";
pub(crate) const ITEM_B: &str = "70a98c94-9252-4fd3-a673-5a584339e273";

/// Builds the bytes of a `.soc` with one shop component for `entity`; `parameters` are the name
/// and value strings that follow the component name.
fn soc(entity: &str, parameters: &[&str]) -> Vec<u8> {
    let mut bytes = b"CrChF\0SCShop\0".to_vec();
    for part in [
        "SCShop",
        entity,
        "EntityComponentShop",
        "SShopComponentParams",
    ] {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(0);
    }
    for part in parameters
        .iter()
        .chain(&["shopInventoryType", "ITEM", "ShopRackGUIDs", "tail"])
    {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(0);
    }
    bytes
}

fn container(stem: &str, entity: &str, parameters: &[&str]) -> Vec<u8> {
    let metadata = TestElement::new("EntityComponentShop")
        .child(TestElement::new("Entity").attribute("CryGUID", entity))
        .to_xml();
    TestContainer::new()
        .entry(TestEntry::stored(
            format!("{stem}\\metadata\\entitycomponentshop.xml"),
            metadata,
        ))
        .entry(TestEntry::stored(
            format!("{stem}.soc"),
            soc(entity, parameters),
        ))
        .build()
}

fn inventory(items: &[&str]) -> Vec<u8> {
    let elements: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                r#"{{"ID":{{"ID":["{item}"]}},"BuyPrice":1.5,"SellPrice":0.0,"CurrentInventory":3.0,"MaxInventory":9.0,"RentalOfferings":[]}}"#
            )
        })
        .collect();
    format!(
        r#"{{"ShopID":"x","Collection":{{"Inventory":[{}]}}}}"#,
        elements.join(",")
    )
    .into_bytes()
}

struct Named {
    folder: &'static str,
    stem: &'static str,
    entity: &'static str,
    property: &'static str,
    path: &'static str,
}

const NAMED: [Named; 6] = [
    Named {
        folder: "service\\none\\",
        stem: "loose_a",
        entity: LOOSE,
        property: "offlineInventoryXMLName",
        path: "Scripts/ShopInventories/Inv_Loose.xml",
    },
    Named {
        folder: "commex\\tdd\\",
        stem: "tdd_test",
        entity: EXCHANGE,
        property: "offlineInventoryJSON",
        path: "Data/scripts/shopinventories/Inv_CommEx.json",
    },
    Named {
        folder: "clothing\\mcraes\\",
        stem: "mcraes_a",
        entity: CLOTHING,
        property: "offlineInventoryXMLName",
        path: "Scripts/ShopInventories/Inv_Clothing_A.xml",
    },
    Named {
        folder: "bar\\one\\",
        stem: "bar_a",
        entity: MIXED,
        property: "offlineInventoryJSON",
        path: "Scripts/ShopInventories/Inv_One.json",
    },
    Named {
        folder: "food\\two\\",
        stem: "food_b",
        entity: MIXED,
        property: "offlineInventoryJSON",
        path: "Scripts/ShopInventories/Inv_Two.json",
    },
    Named {
        folder: "",
        stem: "noodle",
        entity: FOOD,
        property: "offlineInventoryJSON",
        path: "Scripts/ShopInventories/Inv_Noodle.json",
    },
];

/// Writes the Archive into `dir` and returns its path. The exchange names an existing inventory,
/// the clothing shop one the Archive lacks, the noodle shop one that is no JSON, and the two
/// containers of the mixed shop name two different files.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = starmap_fixture::builder().entry(TestEntry::stored(REGISTRY, registry()));
    for named in &NAMED {
        let location = if named.folder.is_empty() {
            format!("{MODULE}shopfood\\{}.socpak", named.stem)
        } else {
            format!("{SHOPS}{}{}.socpak", named.folder, named.stem)
        };
        builder = builder.entry(TestEntry::stored(
            location,
            container(named.stem, named.entity, &[named.property, named.path]),
        ));
    }
    let files = [
        (
            "Inv_CommEx",
            inventory(&[&ITEM_A.to_ascii_uppercase(), ITEM_B, ITEM_A]),
        ),
        ("Inv_Noodle", b"not json".to_vec()),
        (
            "Inv_Loose",
            format!(r#"{{"Inventory":[{{"ID":{{"ID":["{ITEM_A}"]}}}}]}}"#).into_bytes(),
        ),
        ("Inv_One", inventory(&[ITEM_A])),
        ("Inv_Two", inventory(&[ITEM_B])),
    ];
    for (name, bytes) in files {
        builder = builder.entry(TestEntry::stored(
            format!("{INVENTORIES}{name}.json"),
            bytes,
        ));
    }
    builder.write_to(&path)?;
    Ok(path)
}

/// Writes an Archive whose shop container has a `.soc` without any shop component.
pub(crate) fn write_archive_without_components(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let metadata = TestElement::new("EntityComponentShop")
        .child(TestElement::new("Entity").attribute("CryGUID", EXCHANGE))
        .to_xml();
    let container = TestContainer::new()
        .entry(TestEntry::stored(
            "tdd_test\\metadata\\entitycomponentshop.xml",
            metadata,
        ))
        .entry(TestEntry::stored(
            "tdd_test.soc",
            b"CrChF\0nothing".to_vec(),
        ))
        .build();
    starmap_fixture::builder()
        .entry(TestEntry::stored(REGISTRY, registry()))
        .entry(TestEntry::stored(
            format!("{SHOPS}commex\\tdd\\tdd_test.socpak"),
            container,
        ))
        .write_to(&path)?;
    Ok(path)
}
