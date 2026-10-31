// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The soundbank preview over the BNK Entries of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_extraction::{Archive, Bank};
use p4kx_gui::{
    AudioPreview, AudioStatus, Browser, EntryIndex, SoundStatus, SoundbankPreview, SoundbankStatus,
    is_bank_path,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: usize = 40;
const WAIT: Duration = Duration::from_mins(2);

#[derive(Debug, Default)]
struct Tally {
    opened: u32,
    listed: usize,
    prefetched: usize,
    plugin: usize,
    played: u32,
    unsupported: u32,
    other: u32,
}

fn wait_for(browser: &mut Browser, what: &str, busy: impl Fn(&Browser) -> bool) -> TestResult {
    let deadline = Instant::now() + WAIT;
    while busy(browser) {
        if Instant::now() > deadline {
            return Err(format!("{what} did not finish in time").into());
        }
        std::thread::sleep(Duration::from_millis(2));
        browser.poll();
    }
    Ok(())
}

fn opening(browser: &Browser) -> bool {
    browser
        .soundbank()
        .is_some_and(|bank| matches!(bank.status(), SoundbankStatus::Opening))
}

fn decoding(browser: &Browser) -> bool {
    browser
        .soundbank()
        .and_then(SoundbankPreview::audio)
        .is_some_and(AudioPreview::is_busy)
}

/// Returns the ids of the soundbanks that embed a WEM with a readable header: most soundbanks
/// of a real Archive embed none, their sounds are loose WEM Entries.
fn banks_with_sounds(index: &EntryIndex) -> Result<Vec<u32>, Box<dyn Error>> {
    let archive = index.entries_archive()?;
    let mut banks = Vec::new();
    for id in 0..u32::try_from(index.len())? {
        let Some(entry) = index.entry(id).filter(|entry| is_bank_path(entry.path())) else {
            continue;
        };
        let Ok(bank) = Bank::from_archive(archive, entry.path()) else {
            continue;
        };
        if bank.soundbank().wems().any(|wem| wem.parse().is_ok()) {
            banks.push(id);
        }
    }
    Ok(banks)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_soundbanks_of_a_real_archive_list_their_wems_and_play_one() -> TestResult {
    let index = EntryIndex::build(Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?);
    let banks = banks_with_sounds(&index)?;
    assert!(!banks.is_empty(), "the Archive has soundbanks with sounds");
    let step = banks.len().div_ceil(SAMPLES).max(1);
    let mut browser = Browser::new(index);
    let mut tally = Tally::default();

    for id in banks.into_iter().step_by(step) {
        browser.select_entry(id, || {});
        wait_for(&mut browser, "a soundbank", opening)?;
        let Some(SoundbankStatus::Opened(sounds)) =
            browser.soundbank().map(SoundbankPreview::status)
        else {
            return Err(format!("soundbank {id} did not open").into());
        };
        tally.opened += 1;
        tally.listed += sounds.len();
        tally.prefetched += sounds.iter().filter(|sound| sound.is_prefetch()).count();
        tally.plugin += sounds.iter().filter(|sound| sound.is_plugin()).count();
        let Some(first) = sounds.iter().find(|sound| sound.info().is_some()) else {
            continue;
        };
        let first = first.id();
        browser.select_sound(first, || {});
        wait_for(&mut browser, "a WEM", decoding)?;
        match browser
            .soundbank()
            .and_then(SoundbankPreview::audio)
            .map(AudioPreview::status)
        {
            Some(AudioStatus::Opened(_, SoundStatus::Ready(_))) => tally.played += 1,
            Some(AudioStatus::Opened(_, SoundStatus::Unsupported(_))) => tally.unsupported += 1,
            _ => tally.other += 1,
        }
    }

    assert!(tally.listed > 0, "the soundbanks embed WEMs: {tally:?}");
    assert!(tally.played > 0, "a WEM of a soundbank plays: {tally:?}");
    assert_eq!(tally.other, 0, "no WEM fails to decode: {tally:?}");
    Ok(())
}
