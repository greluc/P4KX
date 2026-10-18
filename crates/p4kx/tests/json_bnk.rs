//! The `--json` output of `p4kx bnk`, as snapshots.

use std::error::Error;
use std::path::Path;

use assert_cmd::Command;
use p4kx_testkit::{TestBank, TestCodec, TestWem, test_signal};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn write_bank(dir: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let file = dir.join("SSAM_Test.bnk");
    let bank = TestBank::new(0xD155_6D49)
        .wem(
            100,
            TestWem::pcm(TestCodec::WwisePcm, 2, 48_000, &test_signal(1_200, 2)).bytes(),
        )
        .wem(
            300,
            TestWem::encoded(TestCodec::Vorbis, 1, 44_100, 88_200).bytes(),
        )
        .wem(400, b"RIFF\x04\0\0\0WAVE".to_vec())
        .bytes();
    std::fs::write(&file, bank)?;
    Ok(file)
}

fn json(dir: &Path, args: &[&std::ffi::OsStr]) -> Result<String, Box<dyn Error>> {
    let output = Command::cargo_bin("p4kx")?
        .arg("--json")
        .args(args)
        .output()?;
    let mut value: Value = serde_json::from_slice(&output.stdout)?;
    let prefix = dir.display().to_string();
    redact(&mut value, &prefix);
    Ok(serde_json::to_string_pretty(&value)?)
}

fn redact(value: &mut Value, prefix: &str) {
    match value {
        Value::String(text) => {
            if let Some(rest) = text.strip_prefix(prefix) {
                *text = format!("[DIR]{}", rest.replace('\\', "/"));
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| redact(item, prefix)),
        Value::Object(fields) => fields.values_mut().for_each(|item| redact(item, prefix)),
        _ => {}
    }
}

#[test]
fn bnk_list() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path())?;

    let json = json(
        dir.path(),
        &["bnk".as_ref(), "list".as_ref(), file.as_os_str()],
    )?;

    insta::assert_snapshot!(json);
    Ok(())
}

#[test]
fn bnk_extract() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path())?;
    let out = dir.path().join("out");

    let json = json(
        dir.path(),
        &[
            "bnk".as_ref(),
            "extract".as_ref(),
            file.as_os_str(),
            "--out".as_ref(),
            out.as_os_str(),
        ],
    )?;

    insta::assert_snapshot!(json);
    Ok(())
}
