// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity glb --interior` end to end over the synthetic Entity with an Interior.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, ENTITY_INTERIOR_CLASS, check_glb, entity_interior_entries};

type TestResult = Result<(), Box<dyn Error>>;

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    entity_interior_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn glb(archive: &Path, out: &Path, interior: bool, json: bool) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    command
        .args(["entity", "glb"])
        .arg(archive)
        .arg(ENTITY_INTERIOR_CLASS)
        .arg("--out")
        .arg(out);
    if interior {
        command.arg("--interior");
    }
    let output = command.output()?;
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code(),
    })
}

#[test]
fn without_the_flag_only_the_hull_is_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("hull.glb");

    let run = glb(&archive, &out, false, false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let checked = check_glb(&std::fs::read(&out)?)?;
    assert!(
        checked
            .document
            .nodes()
            .all(|node| node.name() != Some("Interior"))
    );
    assert!(!run.stdout.contains("Interior:"), "{}", run.stdout);
    Ok(())
}

#[test]
fn the_flag_adds_the_interior_below_the_hull_and_lists_what_is_missing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("ship.glb");

    let run = glb(&archive, &out, true, false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let checked = check_glb(&std::fs::read(&out)?)?;
    let hull = checked
        .document
        .nodes()
        .find(|node| node.name() == Some(ENTITY_INTERIOR_CLASS))
        .ok_or("no hull node")?;
    let group = hull
        .children()
        .find(|node| node.name() == Some("Interior"))
        .ok_or("no Interior node below the hull")?;
    assert_eq!(group.children().count(), 6);
    assert!(
        run.stdout.contains("Interior: 6 containers"),
        "{}",
        run.stdout
    );
    for expected in [
        "the container ObjectContainers\\ships\\test\\absent.socpak does not exist",
        "the helper hp_nowhere is no node of the parent's model",
        "is nested in itself and not entered again",
    ] {
        assert!(run.stdout.contains(expected), "{expected}: {}", run.stdout);
    }
    Ok(())
}

#[test]
fn the_json_report_counts_the_interior_only_when_asked() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let without = glb(&archive, &dir.path().join("a.glb"), false, true)?;
    let with = glb(&archive, &dir.path().join("b.glb"), true, true)?;

    let without: serde_json::Value = serde_json::from_str(&without.stdout)?;
    let with: serde_json::Value = serde_json::from_str(&with.stdout)?;
    assert!(without.get("interior").is_none());
    assert_eq!(with["interior"]["containers"], 6);
    assert!(with["interior"]["model_parts"].as_u64().unwrap_or(0) > 0);
    assert!(
        with["issues"]
            .as_array()
            .ok_or("no issues")?
            .iter()
            .any(|issue| issue["kind"] == "container_missing")
    );
    Ok(())
}
