// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The music soundbanks of a real Archive print their object hierarchy with every object
//! decoded, except the types the reader has no decoder for.

use std::error::Error;

use p4kx_extraction::{Archive, Bank, HircKind, HircLayout};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MUSIC_BANK_PREFIX: &str = "data\\sounds\\wwise\\mus_";
const BANK_EXTENSION: &str = ".bnk";
const STANTON_BANK: &str = "Data\\Sounds\\wwise\\MUS_Music_Stanton.bnk";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn music_banks_have_no_unsupported_or_malformed_objects() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let mut banks = 0_usize;
    let mut music_objects = 0_usize;
    for entry in archive.entries().iter().filter(|entry| {
        let path = entry.path().to_ascii_lowercase();
        path.starts_with(MUSIC_BANK_PREFIX) && path.ends_with(BANK_EXTENSION)
    }) {
        let bank = Bank::from_archive(&archive, entry.path())?;
        banks += 1;
        for object in bank.soundbank().objects() {
            assert!(
                matches!(object.layout(), HircLayout::Decoded | HircLayout::Undecoded),
                "{}: object {} ({}) is {:?}: {:?}",
                entry.path(),
                object.id,
                object.kind,
                object.layout(),
                object.error()
            );
            music_objects += usize::from(matches!(
                object.kind,
                HircKind::MusicSegment
                    | HircKind::MusicTrack
                    | HircKind::MusicSwitch
                    | HircKind::MusicPlaylist
            ));
        }
    }
    assert!(banks > 0, "music soundbanks in the Archive");
    assert!(music_objects > 0, "music objects in them");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_music_bank_links_its_switches_segments_and_tracks() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let bank = Bank::from_archive(&archive, STANTON_BANK)?;
    let soundbank = bank.soundbank();

    let segment = soundbank
        .objects()
        .iter()
        .find(|object| object.kind == HircKind::MusicSegment)
        .ok_or("a music segment")?;

    let track_id = segment.children().next().ok_or("a track of the segment")?;
    let track = soundbank.object(track_id).ok_or("the track in the bank")?;
    assert_eq!(track.kind, HircKind::MusicTrack);
    assert_eq!(track.parent_id(), Some(segment.id), "track to segment");
    let parent = segment.parent_id().ok_or("a parent of the segment")?;
    let switch = soundbank.object(parent).ok_or("the parent in the bank")?;
    assert!(
        switch.children().any(|child| child == segment.id),
        "the parent lists the segment"
    );
    Ok(())
}
