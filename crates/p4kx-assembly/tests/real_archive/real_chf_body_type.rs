// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The body type of every CHF of a real Archive goes with the head rig of its DNA.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::io::Read;

use p4kx_assembly::character::{Chf, Guid};
use p4kx_extraction::Archive;
use p4kx_testkit::{FEMALE_BODY_TYPE, MALE_BODY_TYPE, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MALE_RIG: u32 = 0xdd6c_67f6;
const FEMALE_RIG: u32 = 0x9ef4_eb54;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn the_body_type_of_every_shipped_character_goes_with_the_rig_of_its_dna() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let body_types: HashMap<u32, Guid> = [
        (MALE_RIG, Guid::from_stored(MALE_BODY_TYPE)),
        (FEMALE_RIG, Guid::from_stored(FEMALE_BODY_TYPE)),
    ]
    .into_iter()
    .collect();
    let mut variants: HashMap<u32, BTreeSet<u32>> = HashMap::new();
    let mut checked = 0_usize;
    for entry in archive.entries() {
        if !entry.path().to_ascii_lowercase().ends_with(".chf") {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(entry)?.read_to_end(&mut bytes)?;
        let character = Chf::read(&bytes)?.character;
        let Some(dna) = &character.dna else {
            continue;
        };
        let expected = body_types
            .get(&dna.gender_hash)
            .ok_or_else(|| format!("{}: unknown rig {:#010x}", entry.path(), dna.gender_hash))?;
        assert_eq!(character.body_type, *expected, "{}", entry.path());
        variants
            .entry(dna.gender_hash)
            .or_default()
            .insert(dna.variant_hash);
        checked += 1;
    }
    println!(
        "{checked} CHF files with DNA; distinct variant_hash per rig: {:?}",
        variants
            .iter()
            .map(|(rig, hashes)| (format!("{rig:#010x}"), hashes.len()))
            .collect::<Vec<_>>()
    );
    assert!(checked > 0, "the Archive holds CHF files with DNA");
    Ok(())
}
