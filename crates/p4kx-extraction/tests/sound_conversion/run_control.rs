use std::fs;
use std::sync::Mutex;

use p4kx_extraction::{AudioConversion, CancelToken, ConflictPolicy, ExtractOptions, Progress};

use crate::fixture::{BANK, Fixture, PCM, TestResult, VORBIS, XMA, partial_files};

const OGG: &str = "Data/Sounds/wwise/Media/500.ogg";
const EMBEDDED: &str = "Data/Sounds/wwise/Ship/20.wav";

#[test]
fn skip_keeps_existing_converted_files() -> TestResult {
    let fixture = Fixture::sounds()?;
    fixture.run(AudioConversion::Ogg, ConflictPolicy::Skip);
    fs::write(fixture.target.join(OGG), b"old")?;

    let again = fixture.run(AudioConversion::Ogg, ConflictPolicy::Skip);

    assert_eq!(fs::read(fixture.target.join(OGG))?, b"old", "kept");
    assert_eq!(
        again.skipped(),
        [VORBIS, PCM, XMA, BANK],
        "every sound kept"
    );
    Ok(())
}

#[test]
fn if_changed_keeps_unchanged_files_and_replaces_changed_ones() -> TestResult {
    let fixture = Fixture::sounds()?;
    fixture.run(AudioConversion::Ogg, ConflictPolicy::Skip);
    let first = fs::read(fixture.target.join(OGG))?;

    let unchanged = fixture.run(AudioConversion::Ogg, ConflictPolicy::IfChanged);
    fs::write(fixture.target.join(EMBEDDED), b"stale")?;
    let changed = fixture.run(AudioConversion::Ogg, ConflictPolicy::IfChanged);

    assert_eq!(
        unchanged.skipped(),
        [VORBIS, PCM, XMA, BANK],
        "nothing changed"
    );
    assert_eq!(unchanged.converted(), Some(0), "nothing written");
    assert_eq!(
        changed.skipped(),
        [VORBIS, PCM, XMA],
        "only the soundbank is written"
    );
    assert_ne!(
        fs::read(fixture.target.join(EMBEDDED))?,
        b"stale",
        "replaced"
    );
    assert_eq!(fs::read(fixture.target.join(OGG))?, first, "unchanged");
    Ok(())
}

#[test]
fn overwrite_replaces_converted_files() -> TestResult {
    let fixture = Fixture::sounds()?;
    fixture.run(AudioConversion::Ogg, ConflictPolicy::Skip);
    fs::write(fixture.target.join(OGG), b"old")?;
    fs::write(fixture.target.join(EMBEDDED), b"old")?;

    let again = fixture.run(AudioConversion::Ogg, ConflictPolicy::Overwrite);

    assert_ne!(fs::read(fixture.target.join(OGG))?, b"old", "replaced");
    assert_ne!(fs::read(fixture.target.join(EMBEDDED))?, b"old", "replaced");
    assert!(again.skipped().is_empty(), "nothing kept");
    assert_eq!(again.converted(), Some(3), "converted again");
    Ok(())
}

#[test]
fn a_cancelled_run_writes_no_sound_and_leaves_no_partial_file() -> TestResult {
    let fixture = Fixture::sounds()?;
    let cancel = CancelToken::new();
    cancel.cancel();

    let report = fixture.run_with(&ExtractOptions {
        convert: true,
        cancel: Some(&cancel),
        ..ExtractOptions::default()
    });

    assert_eq!(report.cancelled(), [VORBIS, PCM, XMA, BANK], "every sound");
    assert_eq!(report.extracted(), 0, "nothing written");
    assert!(
        partial_files(&fixture.target)?.is_empty(),
        "no partial file"
    );
    Ok(())
}

#[test]
fn cancelling_during_the_run_stops_the_rest_without_partial_files() -> TestResult {
    let fixture = Fixture::sounds()?;
    let cancel = CancelToken::new();
    let on_progress = |progress: Progress| {
        if progress.bytes_done > 0 {
            cancel.cancel();
        }
    };

    let report = fixture.run_with(&ExtractOptions {
        convert: true,
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        ..ExtractOptions::default()
    });

    assert!(!report.cancelled().is_empty(), "the run stopped");
    assert_eq!(
        report.extracted() + report.cancelled().len() + report.failures().len(),
        4,
        "every Entry ended once"
    );
    assert!(
        partial_files(&fixture.target)?.is_empty(),
        "no partial file"
    );
    Ok(())
}

#[test]
fn progress_counts_every_sound_once_and_reaches_the_total() -> TestResult {
    let fixture = Fixture::sounds()?;
    let seen = Mutex::new(Vec::new());
    let on_progress = |progress: Progress| {
        if let Ok(mut seen) = seen.lock() {
            seen.push(progress);
        }
    };

    fixture.run_with(&ExtractOptions {
        convert: true,
        progress: Some(&on_progress),
        ..ExtractOptions::default()
    });

    let seen = seen.into_inner().map_err(|_| "poisoned")?;
    let last = seen.last().copied().ok_or("progress reported")?;
    assert_eq!(last.entries_done, 4, "every Entry ended");
    assert_eq!(last.bytes_done, last.bytes_total, "every byte counted once");
    assert!(
        seen.windows(2)
            .all(|pair| pair[0].bytes_done <= pair[1].bytes_done),
        "never backwards"
    );
    Ok(())
}
