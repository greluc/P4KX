//! Loadouts in the Basetool catalog of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K`
//! names one.

use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::time::{Instant, SystemTime};

use p4kx_assembly::{ArchiveFiles, Attachment, LoadoutResolver, PortNode, PortType};
use p4kx_export::{BuiltCatalog, CatalogOptions, Kind, build_catalog};
use p4kx_extraction::{Archive, DataForgeEntry, TagTree};
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const MIN_PORTS: usize = 10_000;
const ORIG_100I: &str = "6135a874-4cb1-4f49-9f29-5781e5991f2b";
const VEHICLE_ONLY: &str = "vehicle";
const PINNED_100I_TREE_PORTS: usize = 68;

fn list<'c>(catalog: &'c Value, kind: &str) -> Result<&'c [Value], Box<dyn Error>> {
    Ok(catalog[kind].as_array().ok_or("the kind is a list")?)
}

fn ports(entry: &Value) -> &[Value] {
    entry["ports"].as_array().map_or(&[], Vec::as_slice)
}

#[expect(
    clippy::print_stdout,
    reason = "the counts are the result of the verification"
)]
fn report(built: &BuiltCatalog, elapsed: std::time::Duration) {
    let counts = built.summary().loadouts();
    println!("catalog built in {elapsed:?}: {counts:?}");
    println!(
        "catalog size: {} bytes, {:?} compressed",
        built.summary().size(),
        built.summary().compressed_size()
    );
}

fn walk<'c>(ports: &'c [Value], visit: &mut impl FnMut(&'c Value)) {
    for port in ports {
        visit(port);
        walk(self::ports(port), visit);
    }
}

fn check_counts(built: &BuiltCatalog, catalog: &Value) -> TestResult {
    let counts = built.summary().loadouts();
    let loadouts = list(catalog, "loadouts")?;
    assert_eq!(
        built.summary().kind(Kind::Loadouts).map(|kind| kind.count),
        Some(loadouts.len()),
        "the kind summary"
    );
    assert_eq!(
        counts.ships + counts.items,
        loadouts.len(),
        "ships and items"
    );
    assert_eq!(
        counts.ships,
        list(catalog, "ships")?.len(),
        "every ship has a loadout"
    );
    assert!(counts.ports >= MIN_PORTS, "{} ports", counts.ports);
    let (mut ports, mut attached, mut docked, mut unresolved) = (0, 0, 0, 0);
    for entry in loadouts {
        walk(self::ports(entry), &mut |port| {
            ports += 1;
            attached += usize::from(port["unresolved"].is_null() && !port["itemGuid"].is_null());
            docked += usize::from(port["dockedVehicle"] == json!(true));
            unresolved += usize::from(!port["unresolved"].is_null());
        });
    }
    assert_eq!(
        (ports, attached, docked, unresolved),
        (
            counts.ports,
            counts.attached,
            counts.docked_vehicles,
            counts.unresolved
        ),
        "the summary counts the ports of the file"
    );
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the unresolved GUIDs are the result of the verification"
)]
fn check_references(built: &BuiltCatalog, catalog: &Value) -> TestResult {
    let known: HashSet<&str> = ["items", "ships"]
        .into_iter()
        .map(|kind| list(catalog, kind))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter_map(|entity| entity["guid"].as_str())
        .collect();
    let mut owners = HashSet::new();
    let mut outside = HashSet::new();
    let mut classes = HashSet::new();
    for entry in list(catalog, "loadouts")? {
        let owner = entry["entityGuid"].as_str().ok_or("entityGuid")?;
        assert!(known.contains(owner), "owner {owner} is an item or a ship");
        assert!(owners.insert(owner), "{owner} has one loadout");
        walk(ports(entry), &mut |port| {
            if port["unresolved"].is_null()
                && let Some(guid) = port["itemGuid"].as_str()
                && !known.contains(guid)
                && outside.insert(guid)
                && let Some(class) = port["itemClassName"].as_str()
            {
                classes.insert(class);
            }
        });
    }
    assert_eq!(
        outside.len(),
        built.summary().loadouts().outside,
        "the summary counts the Entities without an entry"
    );
    let mut sample: Vec<&str> = classes.into_iter().collect();
    sample.sort_unstable();
    sample.truncate(40);
    println!(
        "{} attached Entities have no entry in items or ships, e.g. {sample:?}",
        outside.len()
    );
    Ok(())
}

fn same(tree: &[PortNode], listed: &[Value], at: &str) {
    assert!(listed.len() >= tree.len(), "{at}: ports");
    for (port, json) in tree.iter().zip(listed) {
        let here = format!("{at}/{}", port.name());
        assert_eq!(json["name"], port.name(), "{here}: name");
        assert_eq!(json["source"], port.source().code(), "{here}: source");
        assert_eq!(
            json["visibility"],
            port.visibility().code(),
            "{here}: visibility"
        );
        if let Some(definition) = port.definition() {
            assert_eq!(json["minSize"], json!(definition.min_size()), "{here}: min");
            assert_eq!(json["maxSize"], json!(definition.max_size()), "{here}: max");
            let types: Vec<&str> = json["types"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|kind| kind["type"].as_str())
                .collect();
            let wanted: Vec<&str> = definition.types().iter().map(PortType::name).collect();
            assert_eq!(types, wanted, "{here}: types");
        }
        match port.attachment() {
            Attachment::Empty => {
                assert!(json["itemGuid"].is_null(), "{here}: empty");
                same(port.nested(), ports(json), &here);
            }
            Attachment::Entity(child) => {
                assert_eq!(json["itemGuid"], child.guid().to_string(), "{here}: item");
                assert_eq!(json["itemClassName"], child.class_name(), "{here}: class");
                assert_eq!(
                    json["dockedVehicle"],
                    json!(child.is_docked_vehicle()),
                    "{here}: docked"
                );
                if child.is_expanded() {
                    assert_eq!(ports(json).len(), child.ports().len(), "{here}: below");
                    same(child.ports(), ports(json), &here);
                } else {
                    assert!(ports(json).is_empty(), "{here}: not expanded");
                }
            }
            Attachment::Unresolved(unresolved) => {
                assert_eq!(
                    json["unresolved"],
                    unresolved.error().kind(),
                    "{here}: error"
                );
            }
        }
    }
}

fn check_known_ship(archive: &Archive, catalog: &Value) -> TestResult {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let record = resolver.entities().find(ORIG_100I)?;
    let tree = resolver.resolve(&record)?;
    let listed = list(catalog, "loadouts")?
        .iter()
        .find(|entry| entry["entityGuid"] == ORIG_100I)
        .ok_or("the loadout of the 100i")?;
    assert_eq!(listed["entityKind"], "ship", "a ship");
    same(tree.ports(), ports(listed), "100i");
    for extra in ports(listed).iter().skip(tree.ports().len()) {
        assert_eq!(extra["source"], VEHICLE_ONLY, "ports beyond the tree");
        assert!(extra["itemGuid"].is_null(), "a vehicle-only port is empty");
    }
    assert_eq!(tree.ports().len(), PINNED_100I_TREE_PORTS, "tree ports");
    let top = ports(listed);
    let by_name = |name: &str| top.iter().find(|port| port["name"] == name);
    let power = by_name("hardpoint_power_plant").ok_or("the power plant port")?;
    assert_eq!(
        power["itemGuid"], "4a4a2a9c-040f-4f36-befe-7101b230fa53",
        "power plant"
    );
    assert_eq!(
        power["itemClassName"], "POWR_LPLT_S01_PowerBolt_SCItem",
        "power plant class"
    );
    let gun = by_name("hardpoint_weapon_gun_left").ok_or("the gun port")?;
    assert_eq!(gun["itemClassName"], "Mount_Gimbal_S3", "gun mount");
    assert_eq!(ports(gun).len(), 1, "the gimbal mount carries one gun port");
    let bay = by_name("hardpoint_weapon_missilebay_front").ok_or("the missile bay")?;
    assert_eq!(
        ports(bay).len(),
        2,
        "the missile rack carries two missile ports"
    );
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the counts are the result of the verification"
)]
fn report_codes(catalog: &Value) -> TestResult {
    let mut unresolved: BTreeMap<&str, usize> = BTreeMap::new();
    let mut issues: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut tag_words, mut tag_nulls) = (0_usize, 0_usize);
    for entry in list(catalog, "loadouts")? {
        for code in entry["issues"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            *issues.entry(code).or_default() += 1;
        }
        walk(ports(entry), &mut |port| {
            if let Some(code) = port["unresolved"].as_str() {
                *unresolved.entry(code).or_default() += 1;
            }
            for (names, guids) in [
                ("portTags", "portTagGuids"),
                ("requiredTags", "requiredTagGuids"),
            ] {
                let (words, found) = (
                    port[names].as_array().map_or(0, Vec::len),
                    port[guids].as_array().map_or(0, Vec::len),
                );
                assert_eq!(
                    words, found,
                    "{names} and {guids} are as long as each other"
                );
                tag_words += words;
                tag_nulls += port[guids]
                    .as_array()
                    .map_or(0, |list| list.iter().filter(|guid| guid.is_null()).count());
            }
        });
    }
    println!("port tag words: {tag_words}, without a tag GUID: {tag_nulls}");
    println!("unresolved ports by code: {unresolved:?}");
    println!("loadouts with an issue, by code: {issues:?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_loadouts_of_a_real_archive_resolve_and_match_the_loadout_tree_of_a_ship() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let started = Instant::now();
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    report(&built, started.elapsed());
    let catalog: Value = serde_json::from_slice(built.json())?;
    check_counts(&built, &catalog)?;
    check_references(&built, &catalog)?;
    report_codes(&catalog)?;
    report_tags(&archive, &catalog)?;
    check_known_ship(&archive, &catalog)
}

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
    let mut ports = Words::default();
    for entry in list(catalog, "loadouts")? {
        walk(self::ports(entry), &mut |port| {
            collect_words(port, "portTags", "portTagGuids", &mut ports);
            collect_words(port, "requiredTags", "requiredTagGuids", &mut ports);
        });
    }
    print_words("port tags", &ports, &named);
    let mut items = Words::default();
    for item in list(catalog, "items")? {
        collect_words(item, "tags", "tagGuids", &mut items);
    }
    print_words("item tags", &items, &named);
    Ok(())
}
