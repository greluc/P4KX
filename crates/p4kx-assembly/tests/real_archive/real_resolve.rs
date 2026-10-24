// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The audio triggers of documented ships of a real Archive, resolved to their sound variants.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_assembly::Entities;
use p4kx_assembly::audio::ArchiveAudio;
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Default)]
struct Tally {
    playing: usize,
    resolved: usize,
    variants: usize,
    breaks: usize,
    unreadable: usize,
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_triggers_of_documented_ships_resolve_to_sound_variants() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let entities = Entities::new(&forge);
    let audio = ArchiveAudio::new(&archive);
    let index = audio.index();
    let started = Instant::now();
    let mut tally = Tally::default();
    for name in [
        "ORIG_890Jump",
        "AEGS_Gladius",
        "ANVL_Hornet_F7C",
        "RSI_Constellation_Andromeda",
        "TMBL_Cyclone",
    ] {
        let triggers = index.entity_audio(&entities.find(name)?)?;
        for trigger in triggers.triggers().iter().filter(|trigger| {
            trigger.unmatched().is_none()
                && !trigger
                    .name()
                    .get(..5)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("stop_"))
        }) {
            let sounds = audio
                .resolve_trigger(trigger.name())
                .ok_or("a matched trigger has a definition")?;
            tally.playing += 1;
            tally.resolved += usize::from(!sounds.variants().is_empty());
            tally.variants += sounds.variants().len();
            tally.breaks += sounds.breaks().len();
            tally.unreadable += sounds
                .breaks()
                .iter()
                .filter(|link| matches!(link.reason().kind(), "malformed" | "unknown_layout"))
                .count();
        }
    }
    report(&tally, started.elapsed());
    assert!(tally.playing > 100, "{tally:?}");
    assert!(
        tally.resolved * 100 >= tally.playing * 95,
        "matched triggers that do not stop a sound reach one: {tally:?}"
    );
    assert_eq!(
        tally.unreadable, 0,
        "every object on the way is read: {tally:?}"
    );
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the share of resolved triggers and the time are the measurement"
)]
fn report(tally: &Tally, elapsed: Duration) {
    println!("{tally:?} in {elapsed:?}");
}
