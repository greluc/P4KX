//! `p4kx chf to-json|from-json` end to end.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::TestChf;
use predicates::prelude::predicate;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join(name);
    fs::write(&path, bytes)?;
    Ok(path)
}

#[test]
fn to_json_then_from_json_gives_the_same_bytes_for_every_supported_version() -> TestResult {
    let dir = tempfile::tempdir()?;
    for test in [TestChf::v7(), TestChf::v8()] {
        let original = test.to_bytes();
        let chf = write_file(dir.path(), "character.chf", &original)?;
        let json = dir.path().join("character.json");
        let back = dir.path().join("back.chf");

        p4kx()?
            .args(["chf", "to-json"])
            .arg(&chf)
            .arg("--out")
            .arg(&json)
            .assert()
            .success();
        p4kx()?
            .args(["chf", "from-json"])
            .arg(&json)
            .arg("--out")
            .arg(&back)
            .assert()
            .success();

        assert_eq!(fs::read(&back)?, original, "v{}", test.male_version);
    }
    Ok(())
}

#[test]
fn to_json_prints_the_json_to_stdout_without_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = write_file(dir.path(), "character.chf", &TestChf::v8().to_bytes())?;

    let output = p4kx()?.args(["chf", "to-json"]).arg(&chf).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let json: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json.pointer("/character/versions/male"),
        Some(&Value::from(8)),
        "male version"
    );
    assert_eq!(
        json.pointer("/character/dna/face_parts/12/part"),
        Some(&Value::from("neck")),
        "neck"
    );
    Ok(())
}

#[test]
fn json_summaries_name_input_output_versions_and_size() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = write_file(dir.path(), "character.chf", &TestChf::v7().to_bytes())?;
    let json = dir.path().join("character.json");
    let back = dir.path().join("back.chf");

    let to = p4kx()?
        .args(["--json", "chf", "to-json"])
        .arg(&chf)
        .arg("--out")
        .arg(&json)
        .output()?;
    let from = p4kx()?
        .args(["--json", "chf", "from-json"])
        .arg(&json)
        .arg("--out")
        .arg(&back)
        .output()?;

    let to: Value = serde_json::from_slice(&to.stdout)?;
    let from: Value = serde_json::from_slice(&from.stdout)?;
    assert_eq!(
        to.get("input"),
        Some(&Value::from(chf.display().to_string())),
        "input"
    );
    assert_eq!(
        to.get("output"),
        Some(&Value::from(json.display().to_string())),
        "output"
    );
    assert_eq!(
        to.get("versions"),
        Some(&serde_json::json!({"female": 2, "male": 7})),
        "versions"
    );
    assert_eq!(from.get("size"), Some(&Value::from(4096)), "size");
    assert_eq!(
        from.get("output"),
        Some(&Value::from(back.display().to_string())),
        "output"
    );
    Ok(())
}

#[test]
fn an_edited_json_is_written_as_a_valid_chf() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = write_file(dir.path(), "character.chf", &TestChf::v7().to_bytes())?;
    let output = p4kx()?.args(["chf", "to-json"]).arg(&chf).output()?;
    let mut json: Value = serde_json::from_slice(&output.stdout)?;
    if let Some(value) = json.pointer_mut("/character/dna/face_parts/0/blends/0/value") {
        *value = Value::from(65535);
    }
    let edited = write_file(dir.path(), "edited.json", json.to_string().as_bytes())?;
    let back = dir.path().join("edited.chf");

    p4kx()?
        .args(["chf", "from-json"])
        .arg(&edited)
        .arg("--out")
        .arg(&back)
        .assert()
        .success();
    let reread = p4kx()?.args(["chf", "to-json"]).arg(&back).output()?;

    let reread: Value = serde_json::from_slice(&reread.stdout)?;
    assert_eq!(
        reread.pointer("/character/dna/face_parts/0/blends/0/value"),
        Some(&Value::from(65535)),
        "the edit"
    );
    Ok(())
}

#[test]
fn a_corrupt_chf_fails_with_exit_code_3_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let test = TestChf {
        crc_override: Some(1),
        ..TestChf::v7()
    };
    let chf = write_file(dir.path(), "corrupt.chf", &test.to_bytes())?;
    let json = dir.path().join("corrupt.json");

    p4kx()?
        .args(["chf", "to-json"])
        .arg(&chf)
        .arg("--out")
        .arg(&json)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("CRC-32C mismatch"));

    assert!(!json.exists(), "no output");
    Ok(())
}

#[test]
fn a_missing_input_fails_with_exit_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;

    p4kx()?
        .args(["chf", "to-json"])
        .arg(dir.path().join("missing.chf"))
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot read"));
    Ok(())
}

#[test]
fn an_unknown_version_or_invalid_json_fails_with_exit_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = write_file(dir.path(), "character.chf", &TestChf::v7().to_bytes())?;
    let output = p4kx()?.args(["chf", "to-json"]).arg(&chf).output()?;
    let mut json: Value = serde_json::from_slice(&output.stdout)?;
    if let Some(male) = json.pointer_mut("/character/versions/male") {
        *male = Value::from(9);
    }
    let unknown = write_file(dir.path(), "unknown.json", json.to_string().as_bytes())?;
    let invalid = write_file(dir.path(), "invalid.json", b"{\"character\": 1}")?;
    let out = dir.path().join("out.chf");

    p4kx()?
        .args(["chf", "from-json"])
        .arg(&unknown)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown male version 9"));
    p4kx()?
        .args(["chf", "from-json"])
        .arg(&invalid)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid CHF JSON"));
    assert!(!out.exists(), "no output");
    Ok(())
}

#[test]
fn to_json_with_json_needs_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = write_file(dir.path(), "character.chf", &TestChf::v7().to_bytes())?;

    p4kx()?
        .args(["--json", "chf", "to-json"])
        .arg(&chf)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--json needs --out"));
    Ok(())
}
