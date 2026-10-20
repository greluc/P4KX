//! `p4kx audio info|wav|ogg` for looping WEMs: the loop points are shown and kept.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, TestCodec, TestEntry, TestVorbis, TestWem, test_signal};
use predicates::prelude::predicate;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const PCM_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\100.wem";
const VORBIS_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\200.wem";
const PLAIN_ENTRY: &str = "Data\\Sounds\\wwise\\Media\\300.wem";

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let signal = test_signal(4_800, 1);
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            PCM_ENTRY,
            TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &signal)
                .smpl_loop(0, 1_200, 3_599)
                .bytes(),
        ))
        .entry(TestEntry::stored(
            VORBIS_ENTRY,
            TestVorbis::new(2, 48_000, &[false; 40])
                .sample_count(4_900)
                .wem()
                .smpl_loop(0, 100, 4_099)
                .bytes(),
        ))
        .entry(TestEntry::stored(
            PLAIN_ENTRY,
            TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &signal).bytes(),
        ))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

fn json_of(command: &mut Command) -> Result<Value, Box<dyn Error>> {
    let output = command.output()?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn info_shows_the_loop_points() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx()?
        .args(["audio", "info"])
        .arg(&archive)
        .arg(PCM_ENTRY)
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(
            r"(?m)^Loop: +frames 1200 to 3600, 2400 frames$",
        )?);
    p4kx()?
        .args(["audio", "info"])
        .arg(&archive)
        .arg(PLAIN_ENTRY)
        .assert()
        .code(0)
        .stdout(predicate::str::is_match(r"(?m)^Loop: +none$")?);
    Ok(())
}

#[test]
fn info_prints_the_loop_as_json() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let looping = json_of(
        p4kx()?
            .args(["--json", "audio", "info"])
            .arg(&archive)
            .arg(VORBIS_ENTRY),
    )?;
    let plain = json_of(
        p4kx()?
            .args(["--json", "audio", "info"])
            .arg(&archive)
            .arg(PLAIN_ENTRY),
    )?;

    assert_eq!(
        looping["loop"],
        json!({ "start": 100, "end": 4_100, "frames": 4_000 }),
        "start, exclusive end and length in frames"
    );
    assert_eq!(plain["loop"], Value::Null, "no loop");
    Ok(())
}

#[test]
fn wav_keeps_the_loop_in_a_smpl_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("100.wav");

    let json = json_of(
        p4kx()?
            .args(["--json", "audio", "wav"])
            .arg(&archive)
            .arg(PCM_ENTRY)
            .arg("--out")
            .arg(&out),
    )?;

    let wav = fs::read(&out)?;
    assert_eq!(wav.get(36..40), Some(&b"smpl"[..]), "a smpl chunk");
    let field = |offset: usize| {
        wav.get(offset..offset + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
    };
    assert_eq!(
        (field(88), field(92)),
        (Some(1_200), Some(3_599)),
        "loop start and inclusive end"
    );
    assert_eq!(json["sample_count"], 4_800, "the sound is written once");
    assert_eq!(
        json["loop"],
        json!({ "start": 1_200, "end": 3_600, "frames": 2_400 }),
        "the loop written"
    );
    Ok(())
}

#[test]
fn ogg_keeps_the_loop_in_comments() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("200.ogg");

    let json = json_of(
        p4kx()?
            .args(["--json", "audio", "ogg"])
            .arg(&archive)
            .arg(VORBIS_ENTRY)
            .arg("--out")
            .arg(&out),
    )?;

    let ogg = fs::read(&out)?;
    let contains = |needle: &[u8]| ogg.windows(needle.len()).any(|window| window == needle);
    assert!(contains(b"LOOPSTART=100"), "LOOPSTART");
    assert!(contains(b"LOOPLENGTH=4000"), "LOOPLENGTH");
    assert_eq!(
        json["loop"],
        json!({ "start": 100, "end": 4_100, "frames": 4_000 }),
        "the loop written"
    );
    Ok(())
}
