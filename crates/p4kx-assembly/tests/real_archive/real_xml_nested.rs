//! XML Loadouts of a real Archive: items without `itemName` that hold nested `<Items>`.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use p4kx_assembly::{ArchiveFiles, Attachment, LoadoutResolver, PortSource};
use p4kx_cryxml::{Element, Xml};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_human_body_port_shows_its_nested_items() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    for name in ["AnimEntity_Human", "AnimEntity_Human_Female"] {
        let entity = resolver.entities().find(name)?;
        let tree = resolver.resolve(&entity)?;
        let body = tree
            .ports()
            .iter()
            .find(|port| port.name() == "Body_ItemPort")
            .ok_or("no Body_ItemPort")?;
        assert!(
            matches!(body.attachment(), Attachment::Empty),
            "{name}: the body port stays empty"
        );
        assert!(!body.nested().is_empty(), "{name}: nested ports");
        let items = tree
            .walk()
            .iter()
            .filter(|(path, port)| {
                path.starts_with("Body_ItemPort/")
                    && port.source() == PortSource::Override
                    && matches!(port.attachment(), Attachment::Entity(_))
            })
            .count();
        assert_eq!(
            items, 10,
            "{name}: items the XML Loadout puts below the body"
        );
    }
    Ok(())
}

#[derive(Debug, Default)]
struct Item {
    port: String,
    name: String,
    children: Vec<Self>,
}

impl Item {
    fn named_below(&self) -> usize {
        self.children
            .iter()
            .map(|child| usize::from(!child.name.is_empty()) + child.named_below())
            .sum()
    }
}

fn items_of_cry(element: Element<'_>) -> Vec<Item> {
    let mut items = Vec::new();
    for child in element.children() {
        match child.name() {
            "Items" => items.extend(items_of_cry(child)),
            "Item" => items.push(Item {
                port: child.attribute("portName").unwrap_or_default().to_owned(),
                name: child.attribute("itemName").unwrap_or_default().to_owned(),
                children: items_of_cry(child),
            }),
            _ => {}
        }
    }
    items
}

fn items_of_text(node: roxmltree::Node<'_, '_>) -> Vec<Item> {
    let mut items = Vec::new();
    for child in node.children().filter(roxmltree::Node::is_element) {
        match child.tag_name().name() {
            "Items" => items.extend(items_of_text(child)),
            "Item" => items.push(Item {
                port: child.attribute("portName").unwrap_or_default().to_owned(),
                name: child.attribute("itemName").unwrap_or_default().to_owned(),
                children: items_of_text(child),
            }),
            _ => {}
        }
    }
    items
}

fn read_items(bytes: &[u8]) -> Option<Vec<Item>> {
    match p4kx_cryxml::read(bytes).ok()? {
        Xml::CryXml(document) => {
            (document.root().name() == "Loadout").then(|| items_of_cry(document.root()))
        }
        Xml::Text(text) => {
            let text = std::str::from_utf8(text).ok()?;
            let document = roxmltree::Document::parse(text.trim_start_matches('\u{feff}')).ok()?;
            (document.root_element().tag_name().name() == "Loadout")
                .then(|| items_of_text(document.root_element()))
        }
    }
}

fn nameless_nested<'i>(items: &'i [Item], out: &mut Vec<&'i Item>, total: &mut usize) {
    for item in items {
        *total += 1;
        if item.name.is_empty() && !item.children.is_empty() {
            out.push(item);
        }
        nameless_nested(&item.children, out, total);
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the counts and example paths are the measurement"
)]
fn count_xml_loadout_items_without_item_name_that_hold_nested_items() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let (mut files, mut total) = (0_usize, 0_usize);
    let mut by_folder: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut examples = Vec::new();
    for entry in archive.entries() {
        let path = entry.path();
        let lower = path.to_ascii_lowercase();
        let is_xml = std::path::Path::new(&lower)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"));
        if !lower.contains("loadouts") || !is_xml || entry.uncompressed_size() > 16 << 20 {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(entry)?.read_to_end(&mut bytes)?;
        let Some(items) = read_items(&bytes) else {
            continue;
        };
        files += 1;
        let mut found = Vec::new();
        nameless_nested(&items, &mut found, &mut total);
        let folder = path.split('\\').take(4).collect::<Vec<_>>().join("\\");
        for item in found {
            let slot = by_folder.entry(folder.clone()).or_default();
            if item.named_below() == 0 {
                slot.0 += 1;
            } else {
                slot.1 += 1;
                if examples.len() < 10 {
                    examples.push(format!("{path} port {}", item.port));
                }
            }
        }
    }
    println!("{files} XML Loadouts, {total} items");
    for (folder, (empty, named)) in &by_folder {
        println!("  {folder}: {empty} nested items all nameless, {named} with named items");
    }
    println!("examples: {examples:#?}");
    assert!(files > 0, "the Archive holds XML Loadouts");
    assert!(
        by_folder.values().any(|(_, named)| *named > 0),
        "LIVE has nameless items with named nested items"
    );
    Ok(())
}
