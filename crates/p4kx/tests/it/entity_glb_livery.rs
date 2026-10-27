// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity glb` with textures and tint palettes over the synthetic ship `TEST_Livery`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry, check_glb, livery_entries};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

fn write_archive(dir: &Path, entries: Vec<TestEntry>) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn glb(archive: &Path, out: &Path, extra: &[&str], json: bool) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    let output = command
        .args(["entity", "glb"])
        .arg(archive)
        .arg("TEST_Livery")
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()?;
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code(),
    })
}

#[test]
fn textures_are_embedded_once_at_the_chosen_mip_and_palettes_are_applied() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), livery_entries())?;
    let out = dir.path().join("ship.glb");

    let run = glb(
        &archive,
        &out,
        &["--materials", "textures", "--texture-mip", "2"],
        true,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(report["textures"], 1, "{report}");
    assert_eq!(report["texture_pixels"], 4, "2 by 2 at mip 2: {report}");
    assert_eq!(report["texture_fallbacks"], serde_json::json!([]));
    assert_eq!(report["palettes"], 7, "every part draws with a palette");
    let kinds: Vec<&str> = report["issues"]
        .as_array()
        .ok_or("issues")?
        .iter()
        .filter_map(|issue| issue["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["palette", "palette"], "lost and child: {report}");
    let checked = check_glb(&std::fs::read(&out)?)?;
    assert_eq!(checked.document.images().len(), 1);
    Ok(())
}

#[test]
fn two_runs_in_textures_mode_write_identical_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), livery_entries())?;
    let (first, second) = (dir.path().join("a.glb"), dir.path().join("b.glb"));

    for out in [&first, &second] {
        let run = glb(&archive, out, &["--materials", "textures"], false)?;
        assert_eq!(run.code, Some(0), "{}", run.stderr);
    }

    let bytes = std::fs::read(&first)?;
    check_glb(&bytes)?;
    assert!(bytes == std::fs::read(&second)?, "byte-identical");
    Ok(())
}

#[test]
fn colours_mode_applies_palettes_without_textures() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), livery_entries())?;
    let out = dir.path().join("ship.glb");

    let run = glb(&archive, &out, &[], true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(report["textures"], 0);
    assert_eq!(report["palettes"], 7);
    let checked = check_glb(&std::fs::read(&out)?)?;
    let red = checked.document.materials().any(|material| {
        let [r, g, b, _] = material.pbr_metallic_roughness().base_color_factor();
        (r - 1.0).abs() < 1e-4 && g.abs() < 1e-4 && b.abs() < 1e-4
    });
    assert!(red, "the ship's primary colour is (255, 0, 0)");
    Ok(())
}

#[test]
fn a_missing_texture_falls_back_to_the_colour_and_is_listed() -> TestResult {
    let entries: Vec<TestEntry> = livery_entries()
        .into_iter()
        .filter(|entry| !format!("{entry:?}").contains("livery_diff"))
        .collect();
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), entries)?;
    let out = dir.path().join("ship.glb");

    let run = glb(&archive, &out, &["--materials", "textures"], false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout.contains(
            "no diffuse texture Objects/Test/livery_diff.tif for material caps: \
             the texture does not exist; the plain colour is used"
        ),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.contains("7 parts drawn with a tint palette"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .contains("note hardpoint_lost (TEST_Lost): the tint palette")
            && run.stdout.contains("does not exist"),
        "{}",
        run.stdout
    );
    let checked = check_glb(&std::fs::read(&out)?)?;
    assert_eq!(checked.document.images().len(), 0);
    Ok(())
}

#[test]
fn a_texture_mip_without_textures_mode_is_accepted_and_changes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), livery_entries())?;
    let (first, second) = (dir.path().join("a.glb"), dir.path().join("b.glb"));

    let plain = glb(&archive, &first, &[], false)?;
    let mipped = glb(&archive, &second, &["--texture-mip", "3"], false)?;

    assert_eq!(plain.code, Some(0), "{}", plain.stderr);
    assert_eq!(mipped.code, Some(0), "{}", mipped.stderr);
    assert_eq!(std::fs::read(&first)?, std::fs::read(&second)?);
    Ok(())
}
