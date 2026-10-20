//! Audio Conversion during an Extraction of a sample of real WEMs and soundbanks.

use std::io::Write;

use p4kx_extraction::{
    Archive, AudioConversion, ConflictPolicy, Entry, ExtractOptions, ExtractionError, extract,
};
use p4kx_testkit::real_archive_path;

use crate::TestResult;

const SOUND_SAMPLE: usize = 300;
const BANK_SAMPLE: usize = 6;
const MAX_BANK_SIZE: u64 = 32 << 20;

fn has_extension(entry: &Entry, extension: &[u8]) -> bool {
    let path = entry.path().as_bytes();
    path.len() > 4
        && path
            .get(path.len() - 4..)
            .is_some_and(|end| end.eq_ignore_ascii_case(extension))
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario over a real Archive")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_real_sounds_and_soundbanks_is_converted_during_an_extraction() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let wems: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| has_extension(entry, b".wem"))
        .collect();
    let banks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| has_extension(entry, b".bnk"))
        .filter(|entry| (1..=MAX_BANK_SIZE).contains(&entry.uncompressed_size()))
        .collect();
    let stride = (wems.len() / SOUND_SAMPLE).max(1);
    let bank_stride = (banks.len() / BANK_SAMPLE).max(1);
    let selected: Vec<&Entry> = wems
        .iter()
        .step_by(stride)
        .chain(banks.iter().step_by(bank_stride))
        .copied()
        .collect();
    let target = tempfile::tempdir()?;
    let options = ExtractOptions {
        convert: true,
        audio: AudioConversion::Ogg,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &selected, target.path(), &options);

    assert!(report.failures().is_empty(), "{:#?}", report.failures());
    let not_decodable = report
        .unconverted()
        .iter()
        .filter(|failure| matches!(failure.error(), ExtractionError::Sound(_)))
        .count();
    writeln!(
        std::io::stderr(),
        "{} sounds and soundbanks extracted, {:?} converted, {not_decodable} sounds not converted",
        report.extracted(),
        report.converted()
    )?;
    assert_eq!(report.extracted(), selected.len(), "every Entry written");
    assert_eq!(
        not_decodable,
        report.unconverted().len(),
        "only sounds are reported"
    );

    let again = extract(
        &archive,
        &selected,
        target.path(),
        &ExtractOptions {
            policy: ConflictPolicy::IfChanged,
            ..options
        },
    );
    assert_eq!(
        again.skipped().len(),
        selected.len(),
        "unchanged files are kept"
    );
    Ok(())
}
