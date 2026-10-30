// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The music soundbanks of a real Archive print their object hierarchy with every object
//! decoded, except the types the reader has no decoder for.

use std::error::Error;

use p4kx_extraction::{Archive, Bank, Entry, HircKind, HircLayout};
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MUSIC_BANK_PREFIX: &str = "data\\sounds\\wwise\\mus_";
const BANK_EXTENSION: &str = ".bnk";

fn music_bank_paths(archive: &Archive) -> Vec<String> {
    archive
        .entries()
        .iter()
        .map(Entry::path)
        .filter(|path| {
            let lower = path.to_ascii_lowercase();
            lower.starts_with(MUSIC_BANK_PREFIX) && lower.ends_with(BANK_EXTENSION)
        })
        .map(str::to_owned)
        .collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn music_banks_have_no_unsupported_or_malformed_objects() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let paths = music_bank_paths(&archive);
    let mut music_objects = 0_usize;
    for path in &paths {
        let bank = Bank::from_archive(&archive, path)?;
        for object in bank.soundbank().objects() {
            assert!(
                matches!(object.layout(), HircLayout::Decoded | HircLayout::Undecoded),
                "{path}: object {} ({}) is {:?}: {:?}",
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
    assert!(!paths.is_empty(), "music soundbanks in the Archive");
    assert!(music_objects > 0, "music objects in them");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn parents_and_children_agree_wherever_both_are_in_the_same_bank() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let (mut segments, mut tracks) = (0_usize, 0_usize);
    for path in music_bank_paths(&archive) {
        let bank = Bank::from_archive(&archive, &path)?;
        let soundbank = bank.soundbank();
        for segment in soundbank
            .objects()
            .iter()
            .filter(|object| object.kind == HircKind::MusicSegment)
        {
            if let Some(parent) = segment
                .parent_id()
                .filter(|parent| *parent != 0)
                .and_then(|parent| soundbank.object(parent))
            {
                segments += 1;
                assert!(
                    parent.children().any(|child| child == segment.id),
                    "{path}: parent {} lists segment {}",
                    parent.id,
                    segment.id
                );
            }
            for track in segment
                .children()
                .filter_map(|child| soundbank.object(child))
            {
                tracks += 1;
                assert_eq!(
                    track.kind,
                    HircKind::MusicTrack,
                    "{path}: child of a segment"
                );
                assert_eq!(
                    track.parent_id(),
                    Some(segment.id),
                    "{path}: track {} to segment",
                    track.id
                );
            }
        }
    }
    assert!(
        segments > 0 && tracks > 0,
        "{segments} segments, {tracks} tracks checked"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_command_prints_every_music_bank_without_unsupported_or_malformed_objects() -> TestResult {
    let archive_path = real_archive_path()?;
    let archive = Archive::open_with_key(&archive_path, Some(real_archive_key()?))?;
    let paths = music_bank_paths(&archive);
    for path in &paths {
        let output = p4kx_testkit::p4kx()?
            .env("P4KX_ARCHIVE_KEY", real_archive_key()?.to_hex())
            .args(["--json", "bnk", "objects"])
            .arg(&archive_path)
            .arg(path)
            .args(["--limit", "10000"])
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{path}: {stderr}");
        let report: Value = serde_json::from_slice(&output.stdout)?;
        let layouts = &report["layouts"];
        assert_eq!(layouts["unsupported"], 0, "{path}: {layouts}");
        assert_eq!(layouts["malformed"], 0, "{path}: {layouts}");
        let bank = Bank::from_archive(&archive, path)?;
        assert_eq!(
            report["total"].as_u64(),
            u64::try_from(bank.soundbank().objects().len()).ok(),
            "{path}: the command lists the objects of the bank"
        );
    }
    assert!(!paths.is_empty(), "music soundbanks in the Archive");
    Ok(())
}
