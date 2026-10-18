//! `p4kx audio wav` and `p4kx bnk extract --format wav` for Wwise Vorbis WEMs end to end.

use std::error::Error;
use std::f64::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, TestBank, TestEntry, TestVorbis};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

const VORBIS_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\500.wem";
const SHORT: u32 = 256;
const PACKETS: usize = 376;
const BIN: u8 = 6;
const TOLERANCE: f64 = 2.0 / 32_768.0;

fn tone() -> TestVorbis {
    TestVorbis::new(2, 48_000, &[false; PACKETS])
        .tone(BIN)
        .sample_count(48_000)
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(VORBIS_ENTRY, tone().wem().bytes()))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(Command::cargo_bin("p4kx")?)
}

/// Returns the interleaved samples of a canonical 44-byte-header PCM WAV.
fn wav_samples(wav: &[u8]) -> Vec<i16> {
    wav.get(44..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(i16::from_le_bytes)
        .collect()
}

/// Returns one hop of the steady tone of spectral line `bin` in every short block: the
/// overlap-add of its windowed inverse MDCT with itself.
fn tone_hop(bin: u8) -> Vec<f64> {
    let n = f64::from(SHORT);
    let block: Vec<f64> = (0..SHORT)
        .map(|i| {
            let i = f64::from(i);
            let window = (PI / 2.0 * ((i + 0.5) / n * PI).sin().powi(2)).sin();
            let phase = 2.0 * PI / n * (i + 0.5 + n / 4.0) * (f64::from(bin) + 0.5);
            window * TestVorbis::tone_level() * phase.cos()
        })
        .collect();
    let (first, second) = block.split_at(block.len() / 2);
    first.iter().zip(second).map(|(a, b)| a + b).collect()
}

/// Asserts that both channels of a stereo WAV are the known tones, line `BIN` and `BIN + 1`.
#[track_caller]
fn assert_known_tone(samples: &[i16]) {
    for channel in 0..2 {
        let hop = tone_hop(BIN + u8::try_from(channel).unwrap_or(0));
        let worst = samples
            .iter()
            .skip(channel)
            .step_by(2)
            .zip(hop.iter().cycle())
            .map(|(&sample, &expected)| (f64::from(sample) / 32_768.0 - expected).abs())
            .fold(0.0, f64::max);
        assert!(
            worst <= TOLERANCE,
            "channel {channel} is the known tone, off by {worst}"
        );
    }
}

#[test]
fn wav_of_a_vorbis_entry_holds_the_known_tone_for_the_sound_s_duration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("500.wav");

    let output = p4kx()?
        .args(["--json", "audio", "wav"])
        .arg(&archive)
        .arg("data/sounds/wwise/media/500.WEM")
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let wav = fs::read(&out)?;
    assert_eq!(json["channels"], 2, "channels");
    assert_eq!(json["sample_rate"], 48_000, "sample rate");
    assert_eq!(json["sample_count"], 48_000, "one second");
    assert_eq!(json["size"], wav.len(), "size");
    let samples = wav_samples(&wav);
    assert_eq!(samples.len(), 2 * 48_000, "every frame");
    assert_known_tone(&samples);
    Ok(())
}

#[test]
fn wav_of_a_corrupt_vorbis_file_names_it_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("damaged.wem");
    let mut bytes = tone().wem().bytes();
    let setup = bytes
        .windows(4)
        .position(|window| window == b"data")
        .ok_or("no data chunk")?
        + 8;
    bytes
        .get_mut(setup..setup + 2)
        .ok_or("no setup")?
        .copy_from_slice(&[0xFF, 0xFF]);
    fs::write(&file, bytes)?;
    let out = dir.path().join("damaged.wav");

    p4kx()?
        .args(["audio", "wav"])
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("damaged.wem"))
        .stderr(predicate::str::contains("the Wwise Vorbis data is invalid"));

    assert!(!out.exists(), "no WAV is written");
    Ok(())
}

#[test]
fn bnk_extract_as_wav_decodes_the_vorbis_wems() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bank = TestBank::new(0xD155_6D49)
        .wem(7, tone().wem().bytes())
        .bytes();
    let file = dir.path().join("SSAM_Tone.bnk");
    fs::write(&file, bank)?;
    let out = dir.path().join("wav");

    p4kx()?
        .args(["bnk", "extract"])
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .args(["--format", "wav"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("extracted 1 of 1 WEMs"));

    let samples = wav_samples(&fs::read(out.join("7.wav"))?);
    assert_eq!(samples.len(), 2 * 48_000, "one second");
    assert_known_tone(&samples);
    Ok(())
}
