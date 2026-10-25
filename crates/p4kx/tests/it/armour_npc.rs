// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx armour glb --loadout` for a story character over the synthetic outfit and head of the
//! testkit: the head and its own skins, the textures option and the animation folders.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Output;

use p4kx_testkit::{ArchiveBuilder, NPC_CLASS, NPC_HEAD_FOLDER, check_glb, npc_entries};

type TestResult = Result<(), Box<dyn Error>>;

fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    npc_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn glb(archive: &Path, out: &Path, extra: &[&str], json: bool) -> Result<Output, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    command
        .args(["armour", "glb"])
        .arg(archive)
        .args(["--loadout", NPC_CLASS, "--out"])
        .arg(out)
        .args(extra);
    Ok(command.output()?)
}

#[test]
fn a_story_character_is_written_with_its_outfit_and_its_own_head() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let out = dir.path().join("npc.glb");

    let run = glb(&archive, &out, &[], false)?;

    let stdout = String::from_utf8(run.stdout)?;
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8(run.stderr)?
    );
    assert!(
        stdout.contains("head npc_test_t1_head on the male body, 3 parts, its own skins in"),
        "{stdout}"
    );
    assert!(
        stdout.contains("part Clothing_Torso_0/TEST_NpcShirt:"),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "part head: {NPC_HEAD_FOLDER}/npc_test_t1_head.skin"
        )),
        "{stdout}"
    );
    check_glb(&std::fs::read(&out)?)?;
    Ok(())
}

#[test]
fn the_json_report_names_the_head_and_the_body_it_gives() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let out = dir.path().join("npc.glb");

    let run = glb(&archive, &out, &["--materials", "textures"], true)?;

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8(run.stderr)?
    );
    let json: serde_json::Value = serde_json::from_slice(&run.stdout)?;
    assert_eq!(json["body"], "male");
    assert_eq!(json["head"]["class_name"], "npc_test_t1_head");
    assert_eq!(json["head"]["bespoke_folder"], NPC_HEAD_FOLDER);
    assert_eq!(json["loadout"]["class_name"], NPC_CLASS);
    Ok(())
}

#[test]
fn an_animation_folder_without_the_animation_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let out = dir.path().join("npc.glb");

    let run = glb(
        &archive,
        &out,
        &[
            "--anim",
            "idle",
            "--anim-folder",
            "Animations/pu/male/nobody",
        ],
        false,
    )?;

    assert_eq!(
        run.status.code(),
        Some(2),
        "{}",
        String::from_utf8(run.stderr)?
    );
    assert!(!out.exists(), "nothing is written");
    Ok(())
}
