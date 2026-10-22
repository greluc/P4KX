//! Where the starmap records that no system container placed appear in the object containers of a
//! real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one. A diagnostic: it asserts
//! nothing about the Archive and prints what it finds.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::io::Read;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, Entry};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MAX_XML_LEN: u64 = 128 << 20;
const HITS_PER_RECORD: usize = 2;
const EXAMPLES: usize = 40;
const LINE_LEN: usize = 260;

type Hits = HashMap<String, Vec<(String, String)>>;

fn main_xml(container: &Archive, socpak: &Entry) -> Option<Vec<u8>> {
    let stem = socpak
        .path()
        .rsplit(['\\', '/'])
        .next()?
        .strip_suffix(".socpak")?
        .to_ascii_lowercase();
    let wanted = format!("{stem}.xml");
    let xml = container.entries().iter().find(|inner| {
        inner
            .path()
            .rsplit(['\\', '/'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(&wanted))
    })?;
    if xml.uncompressed_size() > MAX_XML_LEN {
        return None;
    }
    let mut bytes = Vec::new();
    container
        .open_entry(xml)
        .ok()?
        .take(MAX_XML_LEN)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

fn text_of(bytes: &[u8]) -> Option<String> {
    match cryxml::read(bytes).ok()? {
        Xml::CryXml(document) => {
            let mut out = Vec::new();
            document.write_xml(&mut out).ok()?;
            String::from_utf8(out).ok()
        }
        Xml::Text(text) => std::str::from_utf8(text).ok().map(str::to_owned),
    }
}

fn scan(archive: &Archive, socpak: &Entry, wanted: &HashSet<String>) -> Hits {
    let mut hits = Hits::new();
    let Ok(container) = archive.open_container(socpak) else {
        return hits;
    };
    let Some(text) = main_xml(&container, socpak).and_then(|bytes| text_of(&bytes)) else {
        return hits;
    };
    for line in text.lines().filter(|line| line.contains('"')) {
        for guid in line
            .split('"')
            .filter(|token| token.len() == 36)
            .map(str::to_ascii_lowercase)
            .filter(|token| wanted.contains(token))
        {
            let shown: String = line.trim().chars().take(LINE_LEN).collect();
            hits.entry(guid)
                .or_default()
                .push((socpak.path().to_owned(), shown));
        }
    }
    hits
}

fn unplaced(catalog: &Value) -> Vec<(String, String)> {
    catalog["starmapObjects"]
        .as_array()
        .map(|objects| {
            objects
                .iter()
                .filter(|object| object["position"].is_null())
                .filter_map(|object| {
                    Some((
                        object["guid"].as_str()?.to_owned(),
                        object["className"].as_str()?.to_owned(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn attribute_before(line: &str, guid: &str) -> String {
    let Some(at) = line.to_ascii_lowercase().find(guid) else {
        return "?".to_owned();
    };
    let head = line.get(..at).unwrap_or_default();
    let name = head.trim_end_matches(['"', '=']);
    let element = line.split_whitespace().next().unwrap_or("?");
    format!("{element} {}", name.rsplit(' ').next().unwrap_or_default())
}

#[expect(
    clippy::print_stdout,
    reason = "the diagnostics are the result of the verification"
)]
fn print_hits(missing: &[(String, String)], hits: &Hits) {
    println!(
        "== {} unplaced records, {} appear in some container, {} nowhere",
        missing.len(),
        hits.len(),
        missing
            .iter()
            .filter(|(guid, _)| !hits.contains_key(guid))
            .count()
    );
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    for (guid, lines) in hits {
        for (_, line) in lines {
            *shapes.entry(attribute_before(line, guid)).or_default() += 1;
        }
    }
    println!("== element and attribute that carry the GUID: {shapes:?}");
    let names: HashMap<&str, &str> = missing
        .iter()
        .map(|(guid, name)| (guid.as_str(), name.as_str()))
        .collect();
    for (guid, lines) in hits.iter().take(EXAMPLES) {
        println!("   {guid} {}", names.get(guid.as_str()).unwrap_or(&"?"));
        for (path, line) in lines.iter().take(HITS_PER_RECORD) {
            println!("      {path}\n      {line}");
        }
    }
    let nowhere: Vec<&str> = missing
        .iter()
        .filter(|(guid, _)| !hits.contains_key(guid))
        .map(|(_, name)| name.as_str())
        .take(EXAMPLES)
        .collect();
    println!("== not in any container (first {EXAMPLES}): {nowhere:?}");
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_unplaced_starmap_records_are_looked_for_in_every_object_container() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let missing = unplaced(&catalog);
    let wanted: HashSet<String> = missing.iter().map(|(guid, _)| guid.clone()).collect();
    let sockets: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().to_ascii_lowercase();
            path.ends_with(".socpak") && path.contains("objectcontainers")
        })
        .collect();
    let merged = sockets
        .par_iter()
        .map(|socpak| scan(&archive, socpak, &wanted))
        .reduce(Hits::new, |mut all, part| {
            for (guid, lines) in part {
                all.entry(guid).or_default().extend(lines);
            }
            all
        });
    println_scanned(sockets.len());
    print_hits(&missing, &merged);
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the diagnostics are the result of the verification"
)]
fn println_scanned(count: usize) {
    println!("== scanned {count} object containers");
}
