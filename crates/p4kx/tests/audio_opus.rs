//! `p4kx audio info|ogg|wav` and `p4kx bnk extract --format ogg` for Opus WEMs end to end.

use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use opus_pure::{OggOpusReader, OpusMSDecoder};
use p4kx_testkit::{ArchiveBuilder, TestBank, TestEntry, TestOpus, TestOpusFraming};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

const OPUS_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\600.wem";
const BANK_ENTRY: &str = "Data\\Sounds\\wwise\\Opus.bnk";
const FRAMES: u32 = 24_000;
const MIN_SNR_DB: f64 = 20.0;

const fn sound() -> TestOpus {
    TestOpus::new(TestOpusFraming::Wwise, 2, FRAMES)
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let bank = TestBank::new(1).wem(600, sound().wem()?.bytes()).bytes();
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(OPUS_ENTRY, sound().wem()?.bytes()))
        .entry(TestEntry::zstd(BANK_ENTRY, bank))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

/// Returns the lowest signal-to-noise ratio in dB of any channel of `decoded` against `signal`.
fn worst_snr(signal: &[i16], decoded: &[i16], channels: usize) -> f64 {
    (0..channels)
        .map(|channel| {
            let (mut power, mut noise) = (0.0_f64, 0.0_f64);
            for (expected, got) in signal
                .iter()
                .skip(channel)
                .step_by(channels)
                .zip(decoded.iter().skip(channel).step_by(channels))
            {
                power += f64::from(*expected).powi(2);
                noise += (f64::from(*expected) - f64::from(*got)).powi(2);
            }
            10.0 * (power / noise.max(1.0)).log10()
        })
        .fold(f64::INFINITY, f64::min)
}

/// Decodes an Ogg Opus file as a player would: the pre-skip dropped, the end granule trimmed.
fn play(ogg: Vec<u8>) -> Result<(usize, Vec<i16>), Box<dyn Error>> {
    let mut reader = OggOpusReader::new(Cursor::new(ogg))?;
    let head = reader.head().clone();
    let channels = usize::from(head.channel_count);
    let mut decoder = OpusMSDecoder::new(48_000, channels, head.mapping_family)?;
    let mut samples = Vec::new();
    let mut out = vec![0_i16; 5_760 * channels];
    let mut end = 0;
    while let Some(packet) = reader.read_packet()? {
        let frames = decoder.decode_s16(&packet.data, 5_760, &mut out)?;
        samples.extend_from_slice(out.get(..frames * channels).unwrap_or_default());
        end = usize::try_from(packet.page_granule)?;
    }
    let played = samples
        .get(usize::from(head.pre_skip) * channels..end * channels)
        .ok_or("the end granule lies beyond the packets")?
        .to_vec();
    Ok((channels, played))
}

#[test]
fn info_of_an_opus_entry_names_the_codec_and_the_decoding_rate() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    p4kx()?
        .args(["audio", "info"])
        .arg(&archive)
        .arg(OPUS_ENTRY)
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^Codec: +Opus$")?)
        .stdout(predicate::str::is_match(r"(?m)^Sample rate: +48000 Hz$")?)
        .stdout(predicate::str::is_match(r"(?m)^Duration: +0\.500 s$")?);
    Ok(())
}

#[test]
fn ogg_of_an_opus_entry_is_a_playable_ogg_opus_of_the_known_signal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("600.ogg");

    let output = p4kx()?
        .args(["--json", "audio", "ogg"])
        .arg(&archive)
        .arg(OPUS_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let ogg = fs::read(&out)?;
    assert_eq!(json["codec"], "opus", "codec");
    assert!(json["codebooks"].is_null(), "no codebooks");
    assert_eq!(json["sample_rate"], 48_000, "rate");
    assert_eq!(json["sample_count"], FRAMES, "frames");
    assert_eq!(json["size"], ogg.len(), "size");
    let (channels, played) = play(ogg)?;
    assert_eq!(played.len(), FRAMES as usize * channels, "played frames");
    let snr = worst_snr(&sound().signal(), &played, channels);
    assert!(snr > MIN_SNR_DB, "the known signal: {snr:.1} dB");
    Ok(())
}

#[test]
fn ogg_of_an_opus_entry_says_ogg_opus() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    p4kx()?
        .args(["audio", "ogg"])
        .arg(&archive)
        .arg(OPUS_ENTRY)
        .arg("--out")
        .arg(dir.path().join("600.ogg"))
        .assert()
        .success()
        .stdout(predicate::str::contains("as Ogg Opus"));
    Ok(())
}

#[test]
fn wav_of_an_opus_entry_holds_the_known_signal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("600.wav");

    let output = p4kx()?
        .args(["--json", "audio", "wav"])
        .arg(&archive)
        .arg(OPUS_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let wav = fs::read(&out)?;
    assert_eq!(json["sample_rate"], 48_000, "rate");
    assert_eq!(json["sample_count"], FRAMES, "frames");
    assert_eq!(json["size"], wav.len(), "size");
    let samples: Vec<i16> = wav
        .get(44..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect();
    assert_eq!(samples.len(), FRAMES as usize * 2, "every frame");
    let snr = worst_snr(&sound().signal(), &samples, 2);
    assert!(snr > MIN_SNR_DB, "the known signal: {snr:.1} dB");
    Ok(())
}

#[test]
fn bnk_extract_as_ogg_repackages_an_embedded_opus_wem() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("bank");
    p4kx()?
        .args(["bnk", "extract"])
        .arg(&archive)
        .arg(BANK_ENTRY)
        .args(["--format", "ogg", "--out"])
        .arg(&out)
        .assert()
        .success();
    let (channels, played) = play(fs::read(out.join("600.ogg"))?)?;
    assert_eq!(played.len(), FRAMES as usize * channels, "played frames");
    Ok(())
}
