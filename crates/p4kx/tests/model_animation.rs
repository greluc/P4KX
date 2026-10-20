//! `p4kx model info|glb` with animations from `.caf` files and `.dba` databases, end to end.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CheckedGlb, TestAnimation, TestDba, TestEntry, TestModel, TestSkeleton,
    TestTrack, check_glb,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const SKIN: &str = "Data\\Objects\\Test\\skin.skin";
const SKIN_GEOMETRY: &str = "Data\\Objects\\Test\\skin.skinm";
const DOOR: &str = "Data\\Objects\\Test\\door.cga";
const DOOR_GEOMETRY: &str = "Data\\Objects\\Test\\door.cgam";
const WAVE: &str = "Data\\Animations\\Objects\\Test\\wave.caf";
const NOD: &str = "Data\\Animations\\Objects\\Test\\nod.caf";
const PACK: &str = "Data\\Animations\\Objects\\Test\\pack.dba";
const FAR: &str = "Data\\Animations\\Elsewhere\\far.caf";
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

fn tip() -> u32 {
    TestSkeleton::controller_id(1)
}

fn rotating(name: &str, controller: u32, last_frame: u16) -> TestAnimation {
    TestAnimation::new(name, 30)
        .track(TestTrack::new(controller).rotations(&[(0, IDENTITY), (last_frame, QUARTER_TURN_Z)]))
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let skin = TestModel::two_bone_skin();
    let door = TestModel::cube().with_compact_skeleton(&TestSkeleton::chain(2));
    let pack = TestDba::new()
        .animation(rotating("animations/objects/test/slide.caf", tip(), 60))
        .animation(
            TestAnimation::new("animations/objects/test/slide_back.caf", 30)
                .additive()
                .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY)]))
                .track(TestTrack::new(0xBAD0_0001).rotations(&[(0, IDENTITY)])),
        );
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(SKIN, skin.cgf()))
        .entry(TestEntry::zstd(SKIN_GEOMETRY, skin.cgfm()))
        .entry(TestEntry::zstd(DOOR, door.cgf()))
        .entry(TestEntry::zstd(DOOR_GEOMETRY, door.cgfm()))
        .entry(TestEntry::zstd(WAVE, rotating("wave.caf", tip(), 30).caf()))
        .entry(TestEntry::zstd(NOD, rotating("nod.caf", tip(), 15).caf()))
        .entry(TestEntry::zstd(PACK, pack.bytes()))
        .entry(TestEntry::zstd(FAR, rotating("far.caf", tip(), 90).caf()))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<assert_cmd::Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

fn glb_of(
    archive: &Path,
    out: &Path,
    extra: &[&str],
) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(p4kx()?
        .args(["model", "glb"])
        .arg(archive)
        .arg(SKIN)
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()?)
}

fn animation_names(glb: &CheckedGlb) -> Vec<String> {
    glb.document
        .animations()
        .map(|animation| animation.name().unwrap_or_default().to_owned())
        .collect()
}

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
        text.contains("Animations:  4 (from 3 files)"),
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
    let back = text
        .lines()
        .find(|line| line.starts_with("slide_back "))
        .ok_or("the slide_back row")?;
    assert!(back.contains("yes"), "additive: {back}");
    assert!(
        back.split_whitespace().nth(4) == Some("2") && back.split_whitespace().nth(5) == Some("1"),
        "two tracks, one bound: {back}"
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
    assert_eq!(animations.len(), 4, "nod, wave and the two of the database");
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
        2,
        "two from the database"
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
    assert_eq!(names, ["nod", "slide", "slide_back", "wave"], "all four");
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
    assert_eq!(names, ["slide", "slide_back"], "the database's two");
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
    assert!(stdout.contains("is additive"), "additive note: {stdout}");

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

#[test]
fn anim_from_adds_an_entry_to_look_in() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let without = glb_of(&archive, &out, &["--anim", "far"])?;
    assert_eq!(without.status.code(), Some(2), "not found: {without:?}");

    let with = glb_of(&archive, &out, &["--anim", "far", "--anim-from", FAR])?;
    assert_eq!(with.status.code(), Some(0), "found: {with:?}");
    assert_eq!(
        animation_names(&check_glb(&fs::read(&out)?)?),
        ["far"],
        "from the extra entry"
    );
    Ok(())
}

#[test]
fn anim_from_an_entry_that_does_not_exist_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = glb_of(
        &archive,
        &out,
        &["--anim", "all", "--anim-from", "Data\\nothing.caf"],
    )?;

    assert_eq!(output.status.code(), Some(2), "invalid input: {output:?}");
    Ok(())
}

#[test]
fn a_static_model_moves_the_nodes_its_helper_bones_drive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let door_animation = TestAnimation::new("open.caf", 30).track(
        TestTrack::new(TestSkeleton::controller_id(1))
            .rotations(&[(0, IDENTITY), (30, QUARTER_TURN_Z)]),
    );
    let archive_with_door = dir.path().join("door.p4k");
    let door = TestModel::cube().with_compact_skeleton(&TestSkeleton::chain(2));
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(DOOR, door.cgf()))
        .entry(TestEntry::zstd(DOOR_GEOMETRY, door.cgfm()))
        .entry(TestEntry::zstd(
            "Data\\Animations\\Objects\\Test\\open.caf",
            door_animation.caf(),
        ))
        .write_to(&archive_with_door)?;
    let out = dir.path().join("door.glb");

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(&archive_with_door)
        .arg(DOOR)
        .arg("--out")
        .arg(&out)
        .args(["--anim", "open"])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let glb = check_glb(&fs::read(&out)?)?;
    let channel = glb
        .document
        .animations()
        .next()
        .and_then(|animation| animation.channels().next())
        .ok_or("a channel")?;
    assert_eq!(
        glb.document
            .nodes()
            .nth(channel.target().node().index())
            .and_then(|node| node.name()),
        Some("cube_mesh"),
        "the node bone 1 drives"
    );
    Ok(())
}

#[test]
fn a_model_file_on_disk_finds_the_animations_next_to_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let skin = TestModel::two_bone_skin();
    fs::write(dir.path().join("skin.skin"), skin.cgf())?;
    fs::write(dir.path().join("skin.skinm"), skin.cgfm())?;
    fs::write(
        dir.path().join("wave.caf"),
        rotating("wave.caf", tip(), 30).caf(),
    )?;
    fs::write(
        dir.path().join("nod.caf"),
        rotating("nod.caf", tip(), 15).caf(),
    )?;
    let out = dir.path().join("out.glb");

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(dir.path().join("skin.skin"))
        .arg("--out")
        .arg(&out)
        .args(["--anim", "nod"])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    assert_eq!(
        animation_names(&check_glb(&fs::read(&out)?)?),
        ["nod"],
        "only the one asked for"
    );
    Ok(())
}

#[test]
fn control_characters_of_animation_names_do_not_reach_the_terminal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let skin = TestModel::two_bone_skin();
    let pack = TestDba::new().animation(rotating("animations/\u{1b}[31mred\u{7}.caf", tip(), 3));
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(SKIN, skin.cgf()))
        .entry(TestEntry::zstd(SKIN_GEOMETRY, skin.cgfm()))
        .entry(TestEntry::zstd(
            "Data\\Animations\\Objects\\Test\\evil.dba",
            pack.bytes(),
        ))
        .write_to(&path)?;
    let out = dir.path().join("skin.glb");

    let info = p4kx()?
        .args(["model", "info"])
        .arg(&path)
        .arg(SKIN)
        .output()?;
    assert_eq!(info.status.code(), Some(0), "info: {info:?}");
    assert!(
        !info.stdout.iter().any(|&byte| byte == 0x1b || byte == 0x07),
        "info output has no control characters"
    );

    let glb = glb_of(&path, &out, &["--anim", "all"])?;
    assert_eq!(glb.status.code(), Some(0), "glb: {glb:?}");
    assert!(
        !glb.stdout.iter().any(|&byte| byte == 0x1b || byte == 0x07),
        "glb output has no control characters"
    );
    Ok(())
}
