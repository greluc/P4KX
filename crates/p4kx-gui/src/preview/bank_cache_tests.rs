// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use p4kx_extraction::{Archive, Soundbank};
use p4kx_testkit::{
    ArchiveBuilder, TestBank, TestCodec, TestEntry, TestWem, test_key, test_signal, wwise_short_id,
};
use tempfile::TempDir;

use super::{AudioPreview, BankCache};
use crate::EntryIndex;

type TestResult = Result<(), Box<dyn Error>>;

const BANK_PATH: &str = "Data\\Sounds\\wwise\\Test.bnk";
const WEM: u32 = 100;

fn bank_bytes() -> Vec<u8> {
    let wem = TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(4_800, 1)).bytes();
    TestBank::new(wwise_short_id("Test")).wem(WEM, wem).bytes()
}

fn opened() -> Result<(TempDir, Arc<EntryIndex>), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(BANK_PATH, bank_bytes()))
        .write_to(&path)?;
    let index = EntryIndex::build(Archive::open_with_key(&path, Some(test_key()))?);
    Ok((folder, Arc::new(index)))
}

fn parsed() -> Result<Arc<Soundbank<Vec<u8>>>, Box<dyn Error>> {
    Ok(Arc::new(Soundbank::parse(bank_bytes())?))
}

fn played(preview: &mut AudioPreview) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while preview.player().is_none() && Instant::now() < deadline {
        preview.poll();
        sleep(Duration::from_millis(5));
    }
    preview.player().is_some()
}

#[test]
fn the_cache_keeps_only_the_last_soundbank() -> TestResult {
    let cache = BankCache::default();
    let bank = parsed()?;

    cache.keep(1, &bank);
    cache.keep(2, &bank);

    assert!(cache.get(1).is_none(), "the first was replaced");
    assert!(cache.get(2).is_some(), "the last is kept");
    assert_eq!(cache.kept(), Some(2), "its Entry");
    cache.clear();
    assert!(cache.get(2).is_none(), "cleared");
    Ok(())
}

#[test]
fn a_kept_soundbank_plays_without_reading_its_entry() -> TestResult {
    let (_folder, index) = opened()?;
    let cache = BankCache::default();
    let missing = u32::MAX;
    cache.keep(missing, &parsed()?);

    let mut preview = AudioPreview::start_in_bank(&index, (missing, WEM), &cache, || {});

    assert!(
        played(&mut preview),
        "the kept soundbank decodes: {:?}",
        preview.status()
    );
    Ok(())
}

#[test]
fn a_soundbank_that_was_read_is_kept_for_the_next_sound() -> TestResult {
    let (_folder, index) = opened()?;
    let cache = BankCache::default();
    let id = index.id_of_path(BANK_PATH).ok_or("no soundbank Entry")?;

    let mut first = AudioPreview::start_in_bank(&index, (id, WEM), &cache, || {});

    assert!(played(&mut first), "decodes: {:?}", first.status());
    assert_eq!(cache.kept(), Some(id), "the soundbank is kept");
    Ok(())
}
