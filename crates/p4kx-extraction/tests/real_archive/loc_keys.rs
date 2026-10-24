// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The key spellings of the `global.ini` files of a real Archive: `@blank_space`, `,P` pairs.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_dataforge::{TreeBuilder, Value};
use p4kx_extraction::{Archive, DataForgeEntry, Language, LocalizationSet, Lookup};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const CATALOG_TEXT_FIELDS: [&str; 8] = [
    "Name",
    "Description",
    "displayName",
    "displayDescription",
    "blueprintName",
    "vehicleName",
    "vehicleDescription",
    "description",
];

#[derive(Debug, Default)]
struct Survey<'a> {
    lines: usize,
    blank: Vec<&'a str>,
    suffixes: BTreeMap<&'a str, usize>,
    both: usize,
    differing: Vec<String>,
}

fn survey(text: &str) -> Survey<'_> {
    let mut found = Survey::default();
    let mut plain: BTreeMap<String, &str> = BTreeMap::new();
    let mut suffixed: Vec<(String, &str)> = Vec::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        found.lines += 1;
        if key.eq_ignore_ascii_case("blank_space") {
            found.blank.push(value);
        }
        if let Some((base, suffix)) = key.split_once(',') {
            *found.suffixes.entry(suffix).or_default() += 1;
            suffixed.push((base.trim().to_ascii_lowercase(), value));
        } else {
            plain.insert(key.trim().to_ascii_lowercase(), value);
        }
    }
    for (base, value) in &suffixed {
        if let Some(twin) = plain.get(base) {
            found.both += 1;
            if twin != value {
                found
                    .differing
                    .push(format!("{base}: {twin:?} vs {value:?}"));
            }
        }
    }
    found
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn keys_that_exist_plain_and_comma_suffixed_are_listed_per_language() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let set = LocalizationSet::read_all(&archive)?;
    for language in set.languages() {
        let entry = set.entry(language).ok_or("an entry")?;
        let found = survey(entry.text().text());
        println!(
            "{language}: {} lines, blank_space lines {:?}, suffixes {:?}, plain and suffixed {} \
             (differing {:?})",
            found.lines, found.blank, found.suffixes, found.both, found.differing
        );
        assert!(
            found.differing.is_empty(),
            "{language}: keys that exist plain and with a suffix but with different texts: {:?}",
            found.differing
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn records_use_blank_space_and_it_is_a_placeholder() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let set = LocalizationSet::read_all(&archive)?;
    let forge = DataForgeEntry::read(&archive)?;
    let forge = forge.open()?;
    let mut builder = TreeBuilder::new();
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    for record in forge.records() {
        if let Ok(tree) = builder.build(&record) {
            for field in tree.fields() {
                walk(
                    field.name(),
                    field.value(),
                    record.record_path().unwrap_or(""),
                    &mut uses,
                );
            }
        }
    }
    println!("Record fields holding blank_space: {uses:#?}");
    assert!(!uses.is_empty(), "Records use @blank_space");
    let in_catalog_fields: Vec<&String> = uses
        .keys()
        .filter(|use_| {
            use_.split(':')
                .next()
                .is_some_and(|name| CATALOG_TEXT_FIELDS.contains(&name))
        })
        .collect();
    assert!(
        in_catalog_fields.is_empty(),
        "no catalog name or description field holds @blank_space: {in_catalog_fields:?}"
    );
    assert_eq!(
        set.index()?.lookup("@blank_space", Language::English, None),
        Lookup::Placeholder,
        "@blank_space is a placeholder although the English file holds a text for it"
    );
    Ok(())
}

fn walk(name: &str, value: &Value<'_>, path: &str, uses: &mut BTreeMap<String, usize>) {
    match value {
        Value::Locale(text) | Value::String(text)
            if text
                .trim_start_matches('@')
                .eq_ignore_ascii_case("blank_space") =>
        {
            let folder = path.split('/').take(5).collect::<Vec<_>>().join("/");
            *uses
                .entry(format!("{name}: {text} in {folder}"))
                .or_default() += 1;
        }
        Value::Struct(structure) => {
            for field in structure.fields() {
                walk(field.name(), field.value(), path, uses);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk(name, item, path, uses);
            }
        }
        _ => {}
    }
}
