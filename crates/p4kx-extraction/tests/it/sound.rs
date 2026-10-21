//! Reading WEM sounds from an Archive or from disk and writing them as WAV files.

use std::error::Error;
use std::fs;

use p4kx_extraction::{Archive, AudioError, Codec, ConversionError, Sound};
use p4kx_testkit::{ArchiveBuilder, TestCodec, TestEntry, TestWem, test_signal};

type TestResult = Result<(), Box<dyn Error>>;

const ENTRY: &str = "Data\\Sounds\\wwise\\Media\\12345.wem";

fn pcm_wem() -> Vec<u8> {
    TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(4_800, 1)).bytes()
}

#[test]
fn a_wem_entry_is_found_ignoring_case_and_separators() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(ENTRY, pcm_wem()))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;

    let sound = Sound::from_archive(&archive, "data/sounds/WWISE/media/12345.WEM")?;

    assert_eq!(sound.name(), ENTRY, "the Entry's own path");
    assert_eq!(sound.wem().codec(), Codec::Pcm, "codec");
    assert_eq!(sound.wem().sample_count(), Some(4_800), "frames");
    Ok(())
}

#[test]
fn an_unknown_entry_is_reported_by_its_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(ENTRY, pcm_wem()))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;

    let error = Sound::from_archive(&archive, "Data\\missing.wem").err();

    assert!(
        matches!(error, Some(ConversionError::UnknownEntry(_))),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_wem_file_is_written_as_wav_and_the_input_is_refused_as_output() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("12345.wem");
    fs::write(&input, pcm_wem())?;
    let out = dir.path().join("12345.wav");

    let sound = Sound::from_file(&input)?;
    let report = sound.write_wav_to_file(&out)?;
    let refused = sound.write_wav_to_file(&input).err();

    assert_eq!(report.sample_count, 4_800, "frames");
    assert_eq!(fs::metadata(&out)?.len(), 44 + 9_600, "header and samples");
    assert!(
        matches!(refused, Some(ConversionError::OutputIsInput { .. })),
        "{refused:?}"
    );
    assert_eq!(fs::read(&input)?, pcm_wem(), "the input is untouched");
    Ok(())
}

#[test]
fn a_missing_file_and_a_corrupt_wem_give_typed_errors() -> TestResult {
    let dir = tempfile::tempdir()?;
    let corrupt = dir.path().join("corrupt.wem");
    fs::write(&corrupt, pcm_wem().get(..30).unwrap_or_default())?;

    let missing = Sound::from_file(&dir.path().join("missing.wem")).err();
    let truncated = Sound::from_file(&corrupt).err();

    assert!(
        matches!(missing, Some(ConversionError::NoSuchFile { .. })),
        "{missing:?}"
    );
    assert!(
        matches!(
            truncated,
            Some(ConversionError::Audio {
                source: AudioError::Truncated { .. },
                ..
            })
        ),
        "{truncated:?}"
    );
    Ok(())
}

#[test]
fn an_undecodable_codec_names_it_and_leaves_no_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("aac.wem");
    fs::write(
        &input,
        TestWem::encoded(TestCodec::Tag(0xAAC0), 2, 48_000, 960).bytes(),
    )?;
    let out = dir.path().join("aac.wav");

    let error = Sound::from_file(&input)?.write_wav_to_file(&out).err();

    assert!(
        matches!(
            error,
            Some(ConversionError::Audio {
                source: AudioError::UnsupportedCodec(Codec::Aac),
                ..
            })
        ),
        "{error:?}"
    );
    assert_eq!(
        fs::read_dir(dir.path())?.count(),
        1,
        "only the input is left in the folder"
    );
    Ok(())
}
