// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! An [`ArchiveAudio`] over an index the caller built answers from that index.

use std::error::Error;

use p4kx_assembly::audio::{ArchiveAudio, AudioIndex};
use p4kx_extraction::{Archive, CancelToken};
use p4kx_testkit::{ArchiveBuilder, TestAtl, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn an_audio_over_a_kept_index_uses_it_and_does_not_build_another() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\GameAudio\\SC_Ship.xml",
            TestAtl::new("SC_Ship")
                .trigger("Play_It", &["Play_It_Event"])
                .to_xml(),
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let kept = AudioIndex::build(&archive);

    let audio = ArchiveAudio::with_index(&archive, &kept);

    assert!(
        std::ptr::eq(audio.index(), &raw const kept),
        "the kept index itself"
    );
    assert!(audio.index().trigger("play_it").is_some(), "its triggers");
    Ok(())
}

#[test]
fn building_an_index_stops_once_it_is_cancelled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\GameAudio\\SC_Ship.xml",
            TestAtl::new("SC_Ship")
                .trigger("Play_It", &["Play_It_Event"])
                .to_xml(),
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let cancel = CancelToken::new();

    let live = AudioIndex::build_until(&archive, &cancel);
    cancel.cancel();
    let stopped = AudioIndex::build_until(&archive, &cancel);

    assert!(
        live.is_some_and(|index| index.trigger("play_it").is_some()),
        "an index nobody cancelled"
    );
    assert!(stopped.is_none(), "a cancelled build gives no index");
    Ok(())
}
