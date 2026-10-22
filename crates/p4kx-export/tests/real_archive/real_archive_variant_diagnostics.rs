//! Diagnostics for the variant rules on a real Archive: which armour components the ships carry
//! and which fields differ between item siblings; runs with `--ignored` when `P4KX_DATA_P4K`
//! names one and prints, never asserts.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_extraction::{Archive, DataForgeEntry, RecordTree, TreeBuilder, Value};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const PAIR_ITEMS: [&str; 8] = [
    "vgl_armor_light_helmet_01_01_03",
    "gys_pants_01_01_14",
    "fio_gloves_01_01_09",
    "outlaw_legacy_armor_light_arms_01_05_19",
    "ccc_bear_helmet_01_01_09",
    "dmc_jacket_09_01_16",
    "cba_jacket_01_01_03",
    "gsb_boots_08_01_20",
];
const SIBLINGS: usize = 2;
const SHOWN: usize = 30;

#[expect(
    clippy::print_stdout,
    reason = "the diagnostics are the result of the run"
)]
fn print_line(line: &str) {
    println!("{line}");
}

fn class_of(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(_, class)| class)
}

fn stem(class: &str) -> &str {
    match class.rsplit_once('_') {
        Some((head, tail)) if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) => head,
        _ => class,
    }
}

fn flatten(path: &str, value: &Value<'_>, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Struct(inner) => {
            for field in inner.fields() {
                flatten(&format!("{path}/{}", field.name()), field.value(), out);
            }
        }
        Value::Array(items) => {
            out.insert(format!("{path}#len"), items.len().to_string());
            for (index, item) in items.iter().enumerate() {
                let label = match item {
                    Value::Struct(inner) => inner.type_name().to_owned(),
                    _ => index.to_string(),
                };
                flatten(&format!("{path}[{index}:{label}]"), item, out);
            }
        }
        Value::Reference(reference) => {
            out.insert(path.to_owned(), format!("ref {}", reference.guid()));
        }
        other => {
            out.insert(path.to_owned(), format!("{other:?}"));
        }
    }
}

fn flat(tree: &RecordTree<'_>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for field in tree.fields() {
        flatten(field.name(), field.value(), &mut out);
    }
    out
}

fn print_diff(left: (&str, &BTreeMap<String, String>), right: (&str, &BTreeMap<String, String>)) {
    let mut shown = 0_usize;
    let mut total = 0_usize;
    let keys = left.1.keys().chain(right.1.keys());
    let mut seen = std::collections::BTreeSet::new();
    for key in keys {
        if !seen.insert(key) || left.1.get(key) == right.1.get(key) {
            continue;
        }
        total += 1;
        if shown < SHOWN {
            shown += 1;
            print_line(&format!(
                "    {key}: {} | {}",
                left.1.get(key).map_or("-", String::as_str),
                right.1.get(key).map_or("-", String::as_str)
            ));
        }
    }
    print_line(&format!(
        "  {} vs {}: {total} differing paths",
        left.0, right.0
    ));
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn variant_diagnostics_ship_components_and_armour() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut trees = TreeBuilder::new();
    let mut components: BTreeMap<String, usize> = BTreeMap::new();
    let mut ships = 0_usize;
    for record in forge.records() {
        let Ok(path) = record.record_path() else {
            continue;
        };
        if !path.contains("/entities/spaceships/") && !path.contains("/entities/groundvehicles/") {
            continue;
        }
        let Ok(tree) = trees.build(&record) else {
            continue;
        };
        ships += 1;
        if let Some(Value::Array(list)) = tree.field("Components") {
            for component in list {
                if let Value::Struct(inner) = component {
                    *components.entry(inner.type_name().to_owned()).or_default() += 1;
                }
            }
        }
    }
    print_line(&format!("ship records: {ships}"));
    for (name, count) in &components {
        let lower = name.to_ascii_lowercase();
        if lower.contains("armor") || lower.contains("armour") {
            print_line(&format!("ARMOUR component {name}: {count} ships"));
        }
    }
    print_line("ship component types (name: ships):");
    for (name, count) in &components {
        print_line(&format!("  {name}: {count}"));
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::too_many_lines, reason = "one linear diagnostic run")]
fn variant_diagnostics_item_sibling_diffs() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut trees = TreeBuilder::new();
    let mut by_stem: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut targets: BTreeMap<&str, u32> = BTreeMap::new();
    for (index, record) in forge.records().enumerate() {
        let Ok(name) = record.name() else {
            continue;
        };
        let Ok(path) = record.record_path() else {
            continue;
        };
        if !path.contains("/entities/scitem/") && !path.contains("/entities/legacyitems/") {
            continue;
        }
        let class = class_of(name);
        let Ok(index) = u32::try_from(index) else {
            continue;
        };
        by_stem
            .entry(stem(class).to_ascii_lowercase())
            .or_default()
            .push(index);
        if let Some(wanted) = PAIR_ITEMS
            .iter()
            .find(|item| item.eq_ignore_ascii_case(class))
        {
            targets.insert(wanted, index);
        }
    }
    let records: Vec<_> = forge.records().collect();
    for (wanted, index) in targets {
        let Some(record) = records.get(index as usize) else {
            continue;
        };
        let Ok(tree) = trees.build(record) else {
            continue;
        };
        let left = flat(&tree);
        let siblings = by_stem
            .get(&stem(wanted).to_ascii_lowercase())
            .map(Vec::as_slice)
            .unwrap_or_default();
        print_line(&format!(
            "{wanted}: {} siblings by stem",
            siblings.len().saturating_sub(1)
        ));
        let mut compared = 0_usize;
        for sibling in siblings {
            if *sibling == index || compared >= SIBLINGS {
                continue;
            }
            let Some(other) = records.get(*sibling as usize) else {
                continue;
            };
            let Ok(other_tree) = trees.build(other) else {
                continue;
            };
            compared += 1;
            print_diff(
                (wanted, &left),
                (class_of(other_tree.name()), &flat(&other_tree)),
            );
        }
    }
    Ok(())
}
