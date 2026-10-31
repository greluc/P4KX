// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx armour glb --loadout` for a story character with a face of its own: the expressions of
//! its head and the options `--expression` and `--no-expressions`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, NPC_CLASS, check_glb, npc_face_entries};

type TestResult = Result<(), Box<dyn Error>>;

fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    npc_face_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

/// Runs the command with `extra` and returns the JSON report and the bytes of the GLB.
fn run(extra: &[&str]) -> Result<(serde_json::Value, Vec<u8>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let out = dir.path().join("npc.glb");
    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["armour", "glb"])
        .arg(&archive)
        .args(["--loadout", NPC_CLASS, "--out"])
        .arg(&out)
        .args(extra)
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8(output.stderr)?
    );
    let glb = std::fs::read(&out)?;
    check_glb(&glb)?;
    Ok((serde_json::from_slice(&output.stdout)?, glb))
}

fn mentions(glb: &[u8], name: &str) -> bool {
    glb.windows(name.len())
        .any(|window| window == name.as_bytes())
}

#[test]
fn the_expressions_of_the_head_are_written_by_default() -> TestResult {
    let (_, glb) = run(&[])?;

    assert!(mentions(&glb, "CTL_a") && mentions(&glb, "CTL_b"));
    Ok(())
}

#[test]
fn no_expressions_leaves_the_head_neutral() -> TestResult {
    let (_, glb) = run(&["--no-expressions"])?;

    assert!(!mentions(&glb, "CTL_a") && !mentions(&glb, "CTL_b"));
    Ok(())
}

#[test]
fn a_named_expression_is_the_only_one_written() -> TestResult {
    let (_, glb) = run(&["--expression", "ctl_b"])?;

    assert!(mentions(&glb, "CTL_b"));
    assert!(!mentions(&glb, "CTL_a"));
    Ok(())
}

#[test]
fn an_expression_the_head_lacks_is_an_issue_and_the_file_is_still_written() -> TestResult {
    let (report, _) = run(&["--expression", "nope"])?;

    let kinds: Vec<&str> = report["issues"]
        .as_array()
        .ok_or("issues")?
        .iter()
        .filter_map(|issue| issue["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"unknown_expression"), "{kinds:?}");
    Ok(())
}
