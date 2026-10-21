//! `p4kx model info|glb` with animations from `.caf` files and `.dba` databases, end to end.

use std::error::Error;
use std::fs;

use p4kx_testkit::check_glb;
use serde_json::Value;

use crate::animation_support;

use animation_support::{NOD, PACK, SKIN, animation_names, glb_of, p4kx, write_archive};

type TestResult = Result<(), Box<dyn Error>>;
#[test]
fn info_lists_the_animations_with_their_duration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["model", "info"])
        .arg(&archive)
        .arg(SKIN)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let text = String::from_utf8(output.stdout)?;
    assert!(
        text.contains("Animations:  3 (from 3 files, 1 skipped)"),
        "count: {text}"
    );
    let wave = text
        .lines()
        .find(|line| line.starts_with("wave "))
        .ok_or("the wave row")?;
    assert_eq!(
        wave.split_whitespace().collect::<Vec<_>>()[..8],
        ["wave", "1.000", "30", "30", "1", "1", "2", "no"],
        "name, seconds, frames, rate, tracks, bound, keys, additive: {wave}"
    );
    let slide = text
        .lines()
        .find(|line| line.starts_with("slide "))
        .ok_or("the slide row")?;
    assert!(slide.contains("2.000"), "60 frames at 30 fps: {slide}");
    assert!(
        text.contains(
            "skipped animation slide_back of Data\\Animations\\Objects\\Test\\pack.dba: additive animation \u{2014} needs a base animation"
        ),
        "additive skipped: {text}"
    );
    Ok(())
}

#[test]
fn info_json_gives_every_animation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["--json", "model", "info"])
        .arg(&archive)
        .arg(SKIN)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let info: Value = serde_json::from_slice(&output.stdout)?;
    let animations = info["animations"].as_array().ok_or("animations")?;
    assert_eq!(animations.len(), 3, "nod, wave and the database's slide");
    assert_eq!(
        info["unread_animations"],
        serde_json::json!([{
            "source": PACK,
            "name": "slide_back",
            "reason": "additive animation \u{2014} needs a base animation"
        }]),
        "the additive one is skipped"
    );
    let nod = animations
        .iter()
        .find(|animation| animation["name"] == "nod")
        .ok_or("nod")?;
    assert_eq!(nod["duration_seconds"], 0.5, "15 frames at 30 fps");
    assert_eq!(nod["frames_per_second"], 30, "rate");
    assert_eq!(nod["end_frame"], 15, "end frame");
    assert_eq!(nod["source"], NOD, "source file");
    assert_eq!(nod["bound_tracks"], 1, "bound to the tip");
    assert_eq!(nod["keys"], 2, "keys");
    assert_eq!(
        animations
            .iter()
            .filter(|animation| animation["source"] == PACK)
            .count(),
        1,
        "one from the database"
    );
    Ok(())
}

#[test]
fn glb_includes_the_named_animation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(&archive, &out, &["--anim", "wave"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let glb = check_glb(&fs::read(&out)?)?;
    assert_eq!(animation_names(&glb), ["wave"], "only the one asked for");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("1 animations, 1 channels"),
        "summary: {stdout}"
    );
    Ok(())
}

#[test]
fn glb_without_anim_has_no_animations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(&archive, &out, &[])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    assert_eq!(
        check_glb(&fs::read(&out)?)?.document.animations().len(),
        0,
        "none"
    );
    Ok(())
}

#[test]
fn anim_is_repeatable_matches_ignoring_case_and_reaches_into_databases() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(
        &archive,
        &out,
        &["--anim", "NOD", "--anim", "slide", "--anim", "wave.caf"],
    )?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let mut names = animation_names(&check_glb(&fs::read(&out)?)?);
    names.sort();
    assert_eq!(names, ["nod", "slide", "wave"], "by name, case, file name");
    Ok(())
}

#[test]
fn anim_all_includes_every_animation_found() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(&archive, &out, &["--anim", "all"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let mut names = animation_names(&check_glb(&fs::read(&out)?)?);
    names.sort();
    assert_eq!(names, ["nod", "slide", "wave"], "all but the additive one");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("skipped animation slide_back of")
            && stdout.contains("additive animation \u{2014} needs a base animation"),
        "additive skipped: {stdout}"
    );

    let json = p4kx()?
        .args(["--json", "model", "glb"])
        .arg(&archive)
        .arg(SKIN)
        .arg("--out")
        .arg(&out)
        .args(["--anim", "all"])
        .output()?;
    let summary: Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(
        summary["unread_animations"][0]["name"], "slide_back",
        "listed as skipped"
    );
    Ok(())
}

#[test]
fn anim_with_the_path_of_a_database_includes_all_of_its_animations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(&archive, &out, &["--anim", PACK])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let mut names = animation_names(&check_glb(&fs::read(&out)?)?);
    names.sort();
    assert_eq!(names, ["slide"], "the database's one that is not additive");
    Ok(())
}

#[test]
fn tracks_for_bones_the_skeleton_lacks_are_skipped_and_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(&archive, &out, &["--anim", "slide_back"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(
            "animation slide_back: left out 1 tracks whose bone the skeleton lacks: 0xbad00001"
        ),
        "report: {stdout}"
    );
    assert!(
        stdout.contains("is additive") && stdout.contains("needs a base animation"),
        "additive note: {stdout}"
    );

    let json = p4kx()?
        .args(["--json", "model", "glb"])
        .arg(&archive)
        .arg(SKIN)
        .arg("--out")
        .arg(&out)
        .args(["--anim", "slide_back"])
        .output()?;
    let summary: Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(
        summary["animations"][0]["skipped_tracks"],
        serde_json::json!([0xBAD0_0001_u32]),
        "skipped controller IDs"
    );
    assert_eq!(summary["animations"][0]["channels"], 1, "channels");
    Ok(())
}

#[test]
fn a_name_no_animation_has_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(&archive, &out, &["--anim", "wave", "--anim", "missing"])?;

    assert_eq!(output.status.code(), Some(2), "invalid input: {output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("no animation is named missing"), "{stderr}");
    assert!(!out.exists(), "nothing is written");
    Ok(())
}
