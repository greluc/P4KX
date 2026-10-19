use p4kx_extraction::{AudioConversion, CancelToken, ConflictPolicy, ExtractOptions, Progress};
use p4kx_testkit::{SplitDds, TestEntry, TestTextureFormat};

use crate::fixture::{BANK, Fixture, PCM, TestResult, VORBIS, XMA, bank, partial_files, pcm};

const TEXT: &str = "Data\\Readme.txt";
const HEADER: &str = "Data\\Textures\\rock_diff.dds";

fn mixed() -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut entries = vec![
        TestEntry::stored(TEXT, b"plain".to_vec()),
        TestEntry::zstd(VORBIS, crate::fixture::vorbis()),
        TestEntry::stored(PCM, pcm()),
        TestEntry::zstd(XMA, crate::fixture::xma()),
        TestEntry::zstd(BANK, bank()),
    ];
    entries.extend(
        SplitDds::new(TestTextureFormat::LegacyDxt5, 32, 16)
            .split(3)
            .entries(HEADER),
    );
    Fixture::new(entries)
}

#[test]
fn plain_entries_a_texture_and_sounds_in_one_run_each_end_in_their_own_way() -> TestResult {
    let fixture = mixed()?;

    let report = fixture.run(AudioConversion::Wav, ConflictPolicy::Overwrite);

    assert!(report.failures().is_empty(), "{:?}", report.failures());
    assert_eq!(fixture.read(TEXT)?, b"plain", "the plain Entry");
    assert!(!fixture.read(HEADER)?.is_empty(), "the merged texture");
    assert!(
        fixture.exists("Data\\Sounds\\wwise\\Media\\600.wav"),
        "a sound"
    );
    assert!(
        fixture.exists("Data\\Sounds\\wwise\\Ship\\20.wav"),
        "a WEM of the soundbank"
    );
    assert_eq!(
        report.converted(),
        Some(7),
        "the four Entries of the texture, two sounds and the soundbank"
    );
    Ok(())
}

#[test]
fn cancelling_a_mixed_run_leaves_no_partial_file_and_ends_each_entry_once() -> TestResult {
    let fixture = mixed()?;
    let total = fixture.archive.entries().len();
    let cancel = CancelToken::new();
    let on_progress = |progress: Progress| {
        if progress.bytes_done > 0 {
            cancel.cancel();
        }
    };

    let report = fixture.run_with(&ExtractOptions {
        convert: true,
        audio: AudioConversion::Wav,
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        ..ExtractOptions::default()
    });

    assert!(!report.cancelled().is_empty(), "the run stopped");
    assert!(report.failures().is_empty(), "{:?}", report.failures());
    assert!(
        partial_files(&fixture.target)?.is_empty(),
        "no partial file"
    );
    let cancelled = report
        .cancelled()
        .iter()
        .filter(|path| !path.contains(" WEM "))
        .count();
    assert_eq!(
        report.extracted() + report.skipped().len() + cancelled,
        total,
        "every Entry ended once"
    );
    Ok(())
}
