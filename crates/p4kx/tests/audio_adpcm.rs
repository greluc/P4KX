//! `p4kx audio info|wav` for IMA ADPCM and PTADPCM Entries and files.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, TestEntry, TestWem, test_signal};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

const IMA_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\300.wem";
const PTADPCM_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\400.wem";
const FRAMES: u32 = 24_064;
const MAX_ERROR: i32 = 1_600;

fn ima_wem() -> TestWem {
    TestWem::ima_adpcm(2, 48_000, &test_signal(FRAMES, 2))
}

fn ptadpcm_wem() -> TestWem {
    TestWem::ptadpcm(1, 48_000, 0x24, &test_signal(FRAMES, 1))
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(IMA_ENTRY, ima_wem().bytes()))
        .entry(TestEntry::stored(PTADPCM_ENTRY, ptadpcm_wem().bytes()))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

fn assert_known_signal(wav: &[u8], channels: u16) {
    let decoded: Vec<i16> = wav
        .get(44..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(i16::from_le_bytes)
        .collect();
    let signal = test_signal(FRAMES, channels);
    assert_eq!(decoded.len(), signal.len(), "every frame");
    let error = decoded
        .iter()
        .zip(&signal)
        .map(|(&decoded, &expected)| (i32::from(decoded) - i32::from(expected)).abs())
        .max()
        .unwrap_or(0);
    assert!(error <= MAX_ERROR, "largest error {error}");
}

#[test]
fn info_of_an_ima_entry_names_the_codec_and_the_duration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx()?
        .args(["audio", "info"])
        .arg(&archive)
        .arg(IMA_ENTRY)
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(r"(?m)^Codec: +IMA ADPCM$")?)
        .stdout(predicate::str::is_match(r"(?m)^Channels: +2$")?)
        .stdout(predicate::str::is_match(r"(?m)^Duration: +0\.501 s$")?);
    Ok(())
}

#[test]
fn wav_of_an_ima_entry_holds_the_known_signal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("300.wav");

    p4kx()?
        .args(["audio", "wav"])
        .arg(&archive)
        .arg(IMA_ENTRY)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("bytes"));

    let wav = fs::read(&out)?;
    assert_eq!(wav.get(22..24), Some(&2u16.to_le_bytes()[..]), "stereo");
    assert_known_signal(&wav, 2);
    Ok(())
}

#[test]
fn wav_of_a_ptadpcm_entry_prints_json_and_holds_the_known_signal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("400.wav");

    let output = p4kx()?
        .args(["--json", "audio", "wav"])
        .arg(&archive)
        .arg(PTADPCM_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["channels"], 1, "mono");
    assert_eq!(json["sample_count"], FRAMES, "frames");
    assert_eq!(json["size"], 44 + 2 * FRAMES, "size");
    assert_known_signal(&fs::read(&out)?, 1);
    Ok(())
}

#[test]
fn wav_of_an_adpcm_file_with_a_broken_block_alignment_fails_with_a_typed_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("broken.wem");
    fs::write(&file, ima_wem().block_align(0x47).bytes())?;
    let out = dir.path().join("broken.wav");

    p4kx()?
        .args(["audio", "wav"])
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "the ADPCM block alignment does not fit the channels",
        ));

    assert!(!out.exists(), "no WAV is written");
    Ok(())
}
