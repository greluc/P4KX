// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! How many Wwise events of a real Archive's audio triggers resolve to sounds, and what the
//! broken links that remain say. The counts are printed with `--nocapture`.

#![expect(clippy::print_stdout, reason = "the counts are the report")]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::time::Instant;

use p4kx_assembly::audio::{ArchiveAudio, AtlTrigger, LanguageChoice};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default)]
struct Census {
    triggers_with_sounds: usize,
    events: HashSet<u32>,
    events_in_a_bank: HashSet<u32>,
    variants: HashMap<u32, usize>,
    breaks: HashMap<u32, BTreeSet<&'static str>>,
    break_counts: BTreeMap<String, usize>,
}

impl Census {
    fn merge(mut self, other: Self) -> Self {
        self.triggers_with_sounds += other.triggers_with_sounds;
        self.events.extend(other.events);
        self.events_in_a_bank.extend(other.events_in_a_bank);
        for (event, count) in other.variants {
            *self.variants.entry(event).or_default() += count;
        }
        for (event, kinds) in other.breaks {
            self.breaks.entry(event).or_default().extend(kinds);
        }
        for (kind, count) in other.break_counts {
            *self.break_counts.entry(kind).or_default() += count;
        }
        self
    }
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario over a real Archive")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_events_of_every_trigger_are_followed_and_counted() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let audio = ArchiveAudio::new(&archive);
    let index = audio.index();
    let choice = LanguageChoice::any();
    let started = Instant::now();
    let census = index
        .triggers()
        .par_iter()
        .map(|trigger: &AtlTrigger| {
            let mut census = Census::default();
            for event in trigger.events() {
                census.events.insert(event.id());
                if index.event_soundbanks(event.id()).next().is_some() {
                    census.events_in_a_bank.insert(event.id());
                }
            }
            if let Some(sounds) = audio.resolve_trigger_in(trigger.name(), &choice) {
                census.triggers_with_sounds += usize::from(!sounds.variants().is_empty());
                for variant in sounds.variants() {
                    *census.variants.entry(variant.event().id()).or_default() += 1;
                    let kinds: Vec<&str> =
                        variant.path().iter().map(|step| step.kind.name()).collect();
                    if kinds.iter().any(|kind| kind.starts_with("music")) {
                        *census
                            .break_counts
                            .entry("variants through music".to_owned())
                            .or_default() += 1;
                    }
                    if kinds.iter().skip(1).any(|kind| *kind == "event") {
                        *census
                            .break_counts
                            .entry("variants through PlayEvent".to_owned())
                            .or_default() += 1;
                    }
                }
                for link in sounds.breaks() {
                    let kind = link.reason().kind();
                    census
                        .breaks
                        .entry(link.event().id())
                        .or_default()
                        .insert(kind);
                    let object = link.object().map_or("none", |step| step.kind.name());
                    *census
                        .break_counts
                        .entry(format!("{kind} at {object}"))
                        .or_default() += 1;
                }
            }
            census
        })
        .reduce(Census::default, Census::merge);
    let resolved = census
        .events_in_a_bank
        .iter()
        .filter(|event| census.variants.contains_key(event))
        .count();
    let variants: usize = census.variants.values().sum();
    println!(
        "triggers {}, with sounds {}; events {}, in a soundbank {}, resolved {resolved}; variants \
         {variants}; followed in {:.2?}",
        index.triggers().len(),
        census.triggers_with_sounds,
        census.events.len(),
        census.events_in_a_bank.len(),
        started.elapsed()
    );
    println!(
        "broken links by reason and object: {:?}",
        census.break_counts
    );
    let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
    for event in &census.events_in_a_bank {
        if census.variants.contains_key(event) {
            continue;
        }
        let why = census.breaks.get(event).map_or_else(
            || "no break: no action plays a sound".to_owned(),
            |kinds| kinds.iter().copied().collect::<Vec<_>>().join("+"),
        );
        *unresolved.entry(why).or_default() += 1;
    }
    println!("events in a soundbank without a sound, by break: {unresolved:?}");
    assert!(resolved * 2 > census.events_in_a_bank.len());
    Ok(())
}
