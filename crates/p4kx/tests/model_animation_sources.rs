//! `p4kx model glb` with animations from other sources than the model's folders, and with names
//! and files that must not do harm.

use std::error::Error;
use std::fs;

use p4kx_testkit::{
    ArchiveBuilder, TestAnimation, TestDba, TestEntry, TestModel, TestSkeleton, TestTrack,
    check_glb,
};

mod animation_support;

use animation_support::{
    DOOR, DOOR_GEOMETRY, FAR, IDENTITY, QUARTER_TURN_Z, SKIN, SKIN_GEOMETRY, animation_names,
    glb_of, p4kx, rotating, tip, write_archive,
};

type TestResult = Result<(), Box<dyn Error>>;
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

#[test]
fn an_output_that_is_an_animation_file_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let skin = TestModel::two_bone_skin();
    fs::write(dir.path().join("skin.skin"), skin.cgf())?;
    fs::write(dir.path().join("skin.skinm"), skin.cgfm())?;
    let wave = rotating("wave.caf", tip(), 30).caf();
    let wave_path = dir.path().join("wave.caf");
    fs::write(&wave_path, &wave)?;

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(dir.path().join("skin.skin"))
        .arg("--out")
        .arg(&wave_path)
        .args(["--anim", "wave"])
        .output()?;

    assert_eq!(output.status.code(), Some(2), "invalid input: {output:?}");
    assert_eq!(
        fs::read(&wave_path)?,
        wave,
        "the animation file is untouched"
    );
    Ok(())
}
