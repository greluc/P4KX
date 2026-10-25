// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! How many `HIRC` objects of every real soundbank decode, per object type and bank version, and
//! where the soundbanks a Play action names lie. The counts are printed with `--nocapture`.

#![expect(clippy::print_stdout, reason = "the counts are the report")]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;

use p4kx_extraction::{Archive, Entry, HircKind, HircValue, Soundbank};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

use crate::TestResult;

const DECODED_KINDS: [HircKind; 10] = [
    HircKind::Sound,
    HircKind::Action,
    HircKind::Event,
    HircKind::RandomSequenceContainer,
    HircKind::SwitchContainer,
    HircKind::LayerContainer,
    HircKind::MusicSegment,
    HircKind::MusicTrack,
    HircKind::MusicSwitch,
    HircKind::MusicPlaylist,
];

#[derive(Default)]
struct Outcome {
    decoded: usize,
    raw: usize,
    malformed: usize,
}

#[derive(Default)]
struct Census {
    outcomes: BTreeMap<(u32, String), Outcome>,
    reasons: BTreeMap<String, usize>,
    action_groups: BTreeMap<u16, usize>,
    plays: Vec<Play>,
    objects: HashMap<u32, HashSet<u32>>,
    failures: Vec<String>,
}

#[derive(Clone, Copy)]
struct Play {
    own_bank: u32,
    target: u32,
    named_bank: u32,
    in_own_bank: bool,
}

impl Census {
    fn merge(mut self, other: Self) -> Self {
        for (key, outcome) in other.outcomes {
            let slot = self.outcomes.entry(key).or_default();
            slot.decoded += outcome.decoded;
            slot.raw += outcome.raw;
            slot.malformed += outcome.malformed;
        }
        for (reason, count) in other.reasons {
            *self.reasons.entry(reason).or_default() += count;
        }
        for (group, count) in other.action_groups {
            *self.action_groups.entry(group).or_default() += count;
        }
        self.plays.extend(other.plays);
        for (bank, ids) in other.objects {
            self.objects.entry(bank).or_default().extend(ids);
        }
        self.failures.extend(other.failures);
        self
    }
}

fn census_of(archive: &Archive, entry: &Entry) -> Census {
    let mut census = Census::default();
    let mut bytes = Vec::new();
    let read = archive
        .open_entry(entry)
        .map_err(|error| error.to_string())
        .and_then(|mut reader| {
            reader
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())
        });
    let bank = match read {
        Ok(_) => Soundbank::parse(bytes.as_slice()).map_err(|error| error.to_string()),
        Err(error) => Err(error),
    };
    let bank = match bank {
        Ok(bank) => bank,
        Err(error) => {
            census.failures.push(format!("{}: {error}", entry.path()));
            return census;
        }
    };
    let (version, own) = (bank.header().version, bank.header().id);
    let ids = census.objects.entry(own).or_default();
    for object in bank.objects() {
        ids.insert(object.id);
        let outcome = census
            .outcomes
            .entry((version, format!("{:?}", object.kind)))
            .or_default();
        match &object.value {
            HircValue::Raw => outcome.raw += 1,
            HircValue::Malformed(error) => {
                outcome.malformed += 1;
                *census.reasons.entry(error.to_string()).or_default() += 1;
            }
            _ => outcome.decoded += 1,
        }
        if let HircValue::Action(action) = &object.value {
            *census
                .action_groups
                .entry(action.action_type >> 8)
                .or_default() += 1;
            if action.is_play() {
                census.plays.push(Play {
                    own_bank: own,
                    target: action.target_id,
                    named_bank: action.bank.map_or(0, |bank| bank.bank_id),
                    in_own_bank: false,
                });
            }
        }
    }
    if let Some(own_ids) = census.objects.get(&own) {
        for play in &mut census.plays {
            play.in_own_bank = own_ids.contains(&play.target);
        }
    }
    census
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_hirc_objects_of_every_soundbank_decode_by_type_and_version() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let banks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            entry
                .path()
                .rsplit_once('.')
                .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("bnk"))
        })
        .collect();
    let census = banks
        .par_iter()
        .map(|entry| census_of(&archive, entry))
        .reduce(Census::default, Census::merge);
    assert!(census.failures.is_empty(), "{:#?}", census.failures);

    println!("version  type  decoded  raw  malformed");
    for ((version, kind), outcome) in &census.outcomes {
        println!(
            "{version}  {kind}  {}  {}  {}",
            outcome.decoded, outcome.raw, outcome.malformed
        );
    }
    println!("malformed reasons: {:?}", census.reasons);
    println!("action types by group: {:#04x?}", census.action_groups);
    report_plays(&census);

    for kind in DECODED_KINDS {
        let outcome = census
            .outcomes
            .get(&(154, format!("{kind:?}")))
            .ok_or_else(|| format!("no {kind:?} object in the soundbanks"))?;
        assert_eq!(outcome.malformed, 0, "{kind:?} objects that do not decode");
        assert_eq!(outcome.raw, 0, "{kind:?} objects left raw");
        assert!(outcome.decoded > 0, "{kind:?} objects");
    }
    Ok(())
}

fn report_plays(census: &Census) {
    let banks: HashSet<u32> = census.objects.keys().copied().collect();
    let (mut total, mut named, mut own, mut found_here) = (0_usize, 0_usize, 0_usize, 0_usize);
    let (mut missing_unnamed, mut missing_named_absent) = (0_usize, 0_usize);
    let (mut missing_named_present, mut missing_named_found) = (0_usize, 0_usize);
    for play in &census.plays {
        total += 1;
        named += usize::from(play.named_bank != 0);
        own += usize::from(play.named_bank == play.own_bank);
        if play.in_own_bank {
            found_here += 1;
            continue;
        }
        if play.named_bank == 0 || play.named_bank == play.own_bank {
            missing_unnamed += 1;
        } else if !banks.contains(&play.named_bank) {
            missing_named_absent += 1;
        } else {
            missing_named_present += 1;
            missing_named_found += usize::from(
                census
                    .objects
                    .get(&play.named_bank)
                    .is_some_and(|ids| ids.contains(&play.target)),
            );
        }
    }
    println!(
        "play actions {total}: name a soundbank {named}, name their own {own}, target in own bank \
         {found_here}; target not in own bank: no other bank named {missing_unnamed}, named bank \
         not in the Archive {missing_named_absent}, named bank in the Archive {missing_named_present} \
         (target found there {missing_named_found})"
    );
}
