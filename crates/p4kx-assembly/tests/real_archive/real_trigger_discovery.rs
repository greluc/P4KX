// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Audio trigger discovery in the Records of a real Archive: the triggers named through
//! `GlobalResourceAudio.audioTrigger` against those named by every `audioTrigger` string field of
//! any struct, and against those named by every string field whose name contains `audioTrigger`.
//! The counts are printed with `--nocapture`.

#![expect(clippy::print_stdout, reason = "the counts are the report")]

use std::collections::{BTreeMap, HashSet};
use std::error::Error;

use p4kx_assembly::audio::{ArchiveAudio, TriggerScope, trigger_references_in};
use p4kx_dataforge::{StructValue, TreeBuilder, Value};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const EXACT_FIELD: &str = "audioTrigger";

#[derive(Default)]
struct Found {
    records: usize,
    unreadable: usize,
    exact_string_fields: BTreeMap<String, usize>,
    narrow: HashSet<String>,
    wide: HashSet<String>,
    extras: BTreeMap<String, (HashSet<String>, usize)>,
    entity_extras: HashSet<String>,
}

impl Found {
    fn merge(mut self, other: Self) -> Self {
        self.records += other.records;
        self.unreadable += other.unreadable;
        for (place, count) in other.exact_string_fields {
            *self.exact_string_fields.entry(place).or_default() += count;
        }
        self.narrow.extend(other.narrow);
        self.wide.extend(other.wide);
        for (place, (names, count)) in other.extras {
            let slot = self.extras.entry(place).or_default();
            slot.0.extend(names);
            slot.1 += count;
        }
        self.entity_extras.extend(other.entity_extras);
        self
    }
}

fn exact_string_fields(structure: &StructValue<'_>, places: &mut BTreeMap<String, usize>) {
    for field in structure.fields() {
        if field.name().eq_ignore_ascii_case(EXACT_FIELD)
            && matches!(field.value(), Value::String(_))
        {
            *places.entry(structure.type_name().to_owned()).or_default() += 1;
        }
        exact_value(field.value(), places);
    }
}

fn exact_value(value: &Value<'_>, places: &mut BTreeMap<String, usize>) {
    match value {
        Value::Struct(structure) => exact_string_fields(structure, places),
        Value::Array(items) => {
            for item in items {
                exact_value(item, places);
            }
        }
        _ => {}
    }
}

fn generic_location(location: &str) -> String {
    let mut out = String::with_capacity(location.len());
    let mut inside = false;
    for character in location.chars() {
        match character {
            '[' => {
                inside = true;
                out.push_str("[]");
            }
            ']' => inside = false,
            _ if inside => {}
            _ => out.push(character),
        }
    }
    out
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario over a real Archive")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_audio_trigger_fields_beyond_global_resource_audio_are_counted() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let audio = ArchiveAudio::new(&archive);
    let index = audio.index();
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<_> = forge.records().collect();
    let found = records
        .par_iter()
        .fold(
            || (TreeBuilder::new(), Found::default()),
            |(mut builder, mut found), record| {
                found.records += 1;
                let Ok(tree) = builder.build(record) else {
                    found.unreadable += 1;
                    return (builder, found);
                };
                exact_string_fields(tree.root(), &mut found.exact_string_fields);
                let narrow = trigger_references_in(&tree, TriggerScope::GlobalResourceAudio);
                let known: HashSet<String> = narrow
                    .iter()
                    .map(|reference| reference.name().to_ascii_lowercase())
                    .collect();
                let is_entity = record
                    .type_name()
                    .is_ok_and(|name| name == "EntityClassDefinition");
                for reference in trigger_references_in(&tree, TriggerScope::AudioTriggerFields) {
                    let name = reference.name().to_ascii_lowercase();
                    found.wide.insert(name.clone());
                    if known.contains(&name) {
                        continue;
                    }
                    let place = format!(
                        "{}:{}",
                        record.type_name().unwrap_or("?"),
                        generic_location(reference.location())
                    );
                    let slot = found.extras.entry(place).or_default();
                    slot.0.insert(name.clone());
                    slot.1 += 1;
                    if is_entity {
                        found.entity_extras.insert(name);
                    }
                }
                found.narrow.extend(known);
                (builder, found)
            },
        )
        .map(|(_, found)| found)
        .reduce(Found::default, Found::merge);
    let in_atl = |names: &HashSet<String>| {
        names
            .iter()
            .filter(|name| index.trigger(name).is_some())
            .count()
    };
    let extra: HashSet<String> = found.wide.difference(&found.narrow).cloned().collect();
    println!(
        "records {} ({} unreadable); distinct triggers through GlobalResourceAudio {} ({} in the \
         ATL), through every field named like audioTrigger {} ({} in the ATL); beyond \
         GlobalResourceAudio {} ({} in the ATL), of them in Entity Records {} ({} in the ATL)",
        found.records,
        found.unreadable,
        found.narrow.len(),
        in_atl(&found.narrow),
        found.wide.len(),
        in_atl(&found.wide),
        extra.len(),
        in_atl(&extra),
        found.entity_extras.len(),
        in_atl(&found.entity_extras),
    );
    println!(
        "string fields named exactly audioTrigger, by struct: {:?}",
        found.exact_string_fields
    );
    for (place, (names, count)) in &found.extras {
        println!(
            "beyond: {place}: {} names ({} in the ATL), {count} fields",
            names.len(),
            in_atl(names)
        );
    }
    assert!(
        found
            .exact_string_fields
            .keys()
            .all(|name| name == "GlobalResourceAudio"),
        "an audioTrigger string field outside GlobalResourceAudio: {:?}",
        found.exact_string_fields
    );
    assert!(
        found.wide.is_superset(&found.narrow),
        "the wider rule finds all"
    );
    assert!(
        in_atl(&found.entity_extras) > 0,
        "the wider rule finds real triggers in Entity Records"
    );
    Ok(())
}
