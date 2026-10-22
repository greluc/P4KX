//! A synthetic Archive with a shop registry, shop containers and the starmap of the starmap
//! fixture.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestContainer, TestElement, TestEntry};

use crate::starmap_fixture;

pub(crate) const REGISTRY: &str = "Data\\Libs\\CosmoTools\\LayerRecovery\\LayerBackups.xml";

pub(crate) const EXCHANGE: &str = "42eef9e5-ecd4-6e5b-6254-e332a5dd44b9";
pub(crate) const CLOTHING: &str = "4a000000-0000-4000-8000-0000000000c1";
pub(crate) const FOOD: &str = "4a000000-0000-4000-8000-0000000000f1";
pub(crate) const MIXED: &str = "4a000000-0000-4000-8000-0000000000e1";
pub(crate) const LOOSE: &str = "4a000000-0000-4000-8000-0000000000a1";

const SHOPS: &str = "Data\\ObjectContainers\\PU\\Shops\\";
const MODULE: &str = "Data\\ObjectContainers\\PU\\loc\\mod\\station\\util_a\\int\\module\\";

const ROOT_ID: &str = "4a269932-183e-362d-aacc-188e7debbea9";
const ROOT_NAME: &str = "OOC_Stanton_{183E362D-9932-4A26-A9BE-EB7D8E18CCAA}";
const BODY_ID: &str = "4ce1f681-b479-5269-0536-3d7f02fdccaf";
const BODY_NAME: &str = "OOC_Stanton_1_Hurston_{B4795269-F681-4CE1-AFCC-FD027F3D3605}";

/// One registry entry: the chain below the body, its container names and the shop.
pub(crate) struct Entry<'a> {
    pub(crate) location_id: &'a str,
    pub(crate) location: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) entity: &'a str,
    pub(crate) default_value: &'a str,
    pub(crate) registry_name: &'a str,
}

fn shop(entry: &Entry<'_>) -> TestElement {
    TestElement::new("ShopBackup")
        .attribute(
            "ID",
            format!(
                "{ROOT_ID},{BODY_ID},{},{}",
                entry.location_id, entry.entity_id
            ),
        )
        .attribute(
            "SuperName",
            format!(
                "{ROOT_NAME},{BODY_NAME},{},{}",
                entry.location, entry.entity
            ),
        )
        .attribute("DefaultValue", entry.default_value)
        .attribute("ShopName", entry.registry_name)
}

fn layer(platform: &str, shops: Vec<TestElement>) -> TestElement {
    TestElement::new("LayerBackup")
        .attribute("PlatformName", platform)
        .attribute("LayerName", "Layer0")
        .child(
            shops
                .into_iter()
                .fold(TestElement::new("ShopBackups"), TestElement::child),
        )
        .child(TestElement::new("OrphanedShops"))
}

fn stanton_shops() -> Vec<TestElement> {
    vec![
        shop(&Entry {
            location_id: "41000000-0000-4000-8000-000000000001",
            location: "Hurston_{AAAAAAAA-0000-4000-8000-000000000001}",
            entity_id: EXCHANGE,
            entity: "SCShop_CommEx_TDD_Hurston",
            default_value: "d0000000-0000-4000-8000-000000000001",
            registry_name: "L_TDD_Hurston",
        }),
        shop(&Entry {
            location_id: "41000000-0000-4000-8000-000000000002",
            location: "planet_hurston ",
            entity_id: CLOTHING,
            entity: "SCShop_Clothing_A",
            default_value: "d0000000-0000-4000-8000-000000000002",
            registry_name: "Clothing_A",
        }),
        shop(&Entry {
            location_id: "41000000-0000-4000-8000-000000000003",
            location: "HURSTON",
            entity_id: CLOTHING,
            entity: "SCShop_Clothing_A",
            default_value: "d0000000-0000-4000-8000-000000000002",
            registry_name: "Clothing_A",
        }),
        shop(&Entry {
            location_id: "41000000-0000-4000-8000-000000000004",
            location: "Nowhere_Place_{AAAAAAAA-0000-4000-8000-000000000004}",
            entity_id: FOOD,
            entity: "SCShop_Platinumbay_small_Commodity",
            default_value: "d0000000-0000-4000-8000-000000000004",
            registry_name: "Noodles",
        }),
        shop(&Entry {
            location_id: "41000000-0000-4000-8000-000000000005",
            location: "Hurston",
            entity_id: MIXED,
            entity: "SCShop_Mixed",
            default_value: "d0000000-0000-4000-8000-000000000005",
            registry_name: "Mixed",
        }),
        shop(&Entry {
            location_id: "41000000-0000-4000-8000-000000000001",
            location: "Hurston",
            entity_id: EXCHANGE,
            entity: "SCShop_Duplicate",
            default_value: "",
            registry_name: "Duplicate",
        }),
        TestElement::new("ShopBackup")
            .attribute("ID", format!("{ROOT_ID},{BODY_ID}"))
            .attribute("SuperName", format!("{ROOT_NAME},{BODY_NAME},Short"))
            .attribute("DefaultValue", "")
            .attribute("ShopName", "Malformed"),
    ]
}

/// Returns the registry of the fixture as `CryXml`.
pub(crate) fn registry() -> Vec<u8> {
    let pyro = vec![shop(&Entry {
        location_id: "41000000-0000-4000-8000-000000000006",
        location: "rs_ext_pyro3_leo_{AAAAAAAA-0000-4000-8000-000000000006}",
        entity_id: LOOSE,
        entity: "SCShop_rs_ext_pyro3_LEO_LandingServices",
        default_value: "",
        registry_name: "",
    })];
    TestElement::new("BackupData")
        .child(
            TestElement::new("LayerBackups")
                .child(layer("stantonsystem", stanton_shops()))
                .child(layer("pyrosystem", pyro)),
        )
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn metadata(stem: &str, guids: &[&str]) -> TestEntry {
    let root = guids
        .iter()
        .fold(TestElement::new("EntityComponentShop"), |root, guid| {
            root.child(TestElement::new("Entity").attribute("CryGUID", *guid))
        });
    TestEntry::stored(
        format!("{stem}\\metadata\\entitycomponentshop.xml"),
        root.to_xml(),
    )
}

fn container(stem: &str, guids: &[&str]) -> Vec<u8> {
    TestContainer::new()
        .entry(TestEntry::stored(
            format!("{stem}.xml"),
            TestElement::new("ObjectContainer").to_xml(),
        ))
        .entry(metadata(stem, guids))
        .build()
}

fn exchange_container() -> Vec<u8> {
    let root = TestElement::new("EntityComponentShop")
        .child(TestElement::new("Entity").attribute("CryGUID", EXCHANGE.to_ascii_uppercase()))
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    TestContainer::new()
        .entry(TestEntry::stored(
            "tdd_test\\metadata\\entitycomponentshop.xml",
            root,
        ))
        .build()
}

/// Writes an Archive with the starmap of the starmap fixture and `registry` as the shop registry
/// into `dir` and returns its path.
pub(crate) fn write_archive_with_registry(
    dir: &Path,
    registry: &[u8],
) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    starmap_fixture::builder()
        .entry(TestEntry::stored(REGISTRY, registry.to_vec()))
        .write_to(&path)?;
    Ok(path)
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    starmap_fixture::builder()
        .entry(TestEntry::stored(REGISTRY, registry()))
        .entry(TestEntry::stored(
            format!("{SHOPS}commex\\tdd\\tdd_test.socpak"),
            exchange_container(),
        ))
        .entry(TestEntry::stored(
            format!("{SHOPS}clothing\\mcraes\\mcraes_a.socpak"),
            container("mcraes_a", &[CLOTHING]),
        ))
        .entry(TestEntry::stored(
            format!("{MODULE}shopfood\\noodle.socpak"),
            container("noodle", &[FOOD]),
        ))
        .entry(TestEntry::stored(
            format!("{SHOPS}bar\\one\\bar_a.socpak"),
            container("bar_a", &[MIXED]),
        ))
        .entry(TestEntry::stored(
            format!("{SHOPS}food\\two\\food_b.socpak"),
            container("food_b", &[MIXED]),
        ))
        .entry(TestEntry::stored(
            format!("{SHOPS}service\\none\\no_shop.socpak"),
            container("no_shop", &[]),
        ))
        .entry(TestEntry::stored(
            format!("{SHOPS}admin\\bad\\broken.socpak"),
            b"not a container".to_vec(),
        ))
        .write_to(&path)?;
    Ok(path)
}
