//! How the tag names of ports and items resolve to tag GUIDs in the catalog of a real Archive;
//! runs with `--ignored` when `P4KX_DATA_P4K` names one. It prints and asserts little: it is the
//! diagnosis of the unresolved names.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::{Archive, DataForgeEntry, TagTree};
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

use super::real_archive_loadouts::{list, ports, walk};

type TestResult = Result<(), Box<dyn Error>>;

const TOP_UNRESOLVED: usize = 40;
const SAMPLE_TAGS: usize = 15;
const SAMPLE_AMBIGUOUS: usize = 5;

#[derive(Default)]
struct Words<'c> {
    total: usize,
    unresolved: BTreeMap<&'c str, usize>,
}

fn collect_words<'c>(value: &'c Value, names: &str, guids: &str, into: &mut Words<'c>) {
    let (Some(names), Some(guids)) = (value[names].as_array(), value[guids].as_array()) else {
        return;
    };
    for (name, guid) in names.iter().zip(guids) {
        into.total += 1;
        if guid.is_null()
            && let Some(name) = name.as_str()
        {
            *into.unresolved.entry(name).or_default() += 1;
        }
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the tag resolution is the result of the diagnosis"
)]
fn print_words(label: &str, words: &Words<'_>, named: &BTreeMap<String, Vec<String>>) {
    let unresolved: usize = words.unresolved.values().sum();
    let ambiguous: usize = words
        .unresolved
        .iter()
        .filter(|(name, _)| {
            named
                .get(&name.to_ascii_lowercase())
                .is_some_and(|tags| tags.len() > 1)
        })
        .map(|(_, count)| *count)
        .sum();
    let known_only_by_case_or_path: usize = words
        .unresolved
        .iter()
        .filter(|(name, _)| named.contains_key(&name.to_ascii_lowercase()))
        .map(|(_, count)| *count)
        .sum();
    println!(
        "{label}: {} words, {unresolved} without a GUID, of which {ambiguous} ambiguous (several tags of that name), {} not in the tag tree by name; {} distinct unresolved names, {known_only_by_case_or_path} have a tag of that name",
        words.total,
        unresolved - known_only_by_case_or_path,
        words.unresolved.len(),
    );
    let mut top: Vec<(&&str, &usize)> = words.unresolved.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (name, count) in top.into_iter().take(TOP_UNRESOLVED) {
        let tags = named.get(&name.to_ascii_lowercase()).map_or(0, Vec::len);
        println!("  {count:>7}  {name:?}  tags of that name: {tags}");
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the tag resolution is the result of the diagnosis"
)]
fn report_tags(archive: &Archive, catalog: &Value) -> TestResult {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let tree = TagTree::build(&forge);
    let mut named: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for tag in tree.iter() {
        named
            .entry(tag.name().to_ascii_lowercase())
            .or_default()
            .push(tag.path().to_owned());
    }
    let ambiguous = named.values().filter(|paths| paths.len() > 1).count();
    println!(
        "tag tree: {} tags, {} distinct lower-case names, {ambiguous} names of more than one tag, issues {:?}",
        tree.len(),
        named.len(),
        tree.issues()
    );
    for tag in tree.iter().take(SAMPLE_TAGS) {
        println!(
            "  tag {:?} path {:?} guid {}",
            tag.name(),
            tag.path(),
            tag.guid()
        );
    }
    for (name, paths) in named
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .take(SAMPLE_AMBIGUOUS)
    {
        println!(
            "  ambiguous {name:?}: {:?}",
            paths.iter().take(4).collect::<Vec<_>>()
        );
    }
    let mut port_words = Words::default();
    for entry in list(catalog, "loadouts")? {
        walk(ports(entry), &mut |port| {
            collect_words(port, "portTags", "portTagGuids", &mut port_words);
            collect_words(port, "requiredTags", "requiredTagGuids", &mut port_words);
        });
    }
    print_words("port tags", &port_words, &named);
    let mut items = Words::default();
    for item in list(catalog, "items")? {
        collect_words(item, "tags", "tagGuids", &mut items);
    }
    print_words("item tags", &items, &named);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_tag_names_of_ports_and_items_resolve_to_tag_guids() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    report_tags(&archive, &catalog)
}
