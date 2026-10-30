// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Census of the Entry paths, Record names and record paths of a real Archive that are no file
//! name valid on both Linux and Windows.

use std::collections::{BTreeMap, HashMap};

use p4kx_extraction::{Archive, DataForgeEntry, NameRule, check_part};
use p4kx_testkit::real_archive_path;

use super::{TestResult, report};

const EXAMPLES: usize = 5;

/// The kind of a broken rule, without its detail.
fn kind(rule: NameRule) -> String {
    match rule {
        NameRule::ForbiddenCharacter(character) => format!("character {character:?}"),
        other => format!("{other:?}"),
    }
}

fn first_failure<'a>(path: &'a str, separators: &[char]) -> Option<(&'a str, NameRule)> {
    path.split(separators)
        .find_map(|part| check_part(part).err().map(|rule| (part, rule)))
}

#[derive(Default)]
struct Census {
    total: usize,
    failing: usize,
    by_kind: BTreeMap<String, usize>,
    examples: Vec<String>,
}

impl Census {
    fn count(&mut self, text: &str, failure: Option<(&str, NameRule)>) {
        self.total += 1;
        let Some((part, rule)) = failure else {
            return;
        };
        self.failing += 1;
        *self.by_kind.entry(kind(rule)).or_default() += 1;
        if self.examples.len() < EXAMPLES {
            self.examples
                .push(format!("{text:?} part {part:?}: {rule}"));
        }
    }

    fn report(&self, what: &str) {
        report(&format!(
            "{what}: {} of {} fail, by kind {:?}, e.g. {:?}",
            self.failing, self.total, self.by_kind, self.examples
        ));
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn counts_the_names_of_a_real_archive_that_are_not_portable() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;

    let mut entries = Census::default();
    let mut folded: HashMap<String, usize> = HashMap::new();
    for entry in archive.entries() {
        entries.count(entry.path(), first_failure(entry.path(), &['\\', '/']));
        *folded
            .entry(entry.path().replace('/', "\\").to_lowercase())
            .or_default() += 1;
    }
    entries.report("Entry paths");
    let collisions = folded.values().filter(|count| **count > 1).count();
    report(&format!(
        "Entry paths that differ only in case or repeat: {collisions} groups"
    ));

    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut names = Census::default();
    let mut paths = Census::default();
    let mut percent = 0;
    for record in forge.records() {
        if let Ok(name) = record.name() {
            names.count(name, check_part(name).err().map(|rule| (name, rule)));
        }
        if let Ok(path) = record.record_path() {
            paths.count(path, first_failure(path, &['/']));
            percent += usize::from(path.contains('%'));
        }
    }
    names.report("Record names");
    paths.report("Record paths");
    report(&format!("Record paths with a percent sign: {percent}"));
    Ok(())
}
