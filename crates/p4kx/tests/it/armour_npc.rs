// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx armour glb --loadout` for a story character over the synthetic outfit and head of the
//! testkit: the head and its own skins, the textures option and the animation folders.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Output;

use p4kx_testkit::{
    ArchiveBuilder, HEAD_REGIONS, HEAD_VERTICES, MALE_HEAD_FOLDER, MALE_RIG, MALE_RIG_HASH,
    NPC_CLASS, NPC_HEAD_FOLDER, TestAnimation, TestChf, TestEntry, TestSkeleton, TestTrack,
    check_glb, head_rig, npc_entries,
};

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
        stdout.contains("head npc_test_t1_head on the male body, 4 parts, its own skins in"),
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

fn clip(name: &str) -> TestAnimation {
    TestAnimation::new(name, 30).track(
        TestTrack::new(TestSkeleton::controller_id(2))
            .rotations(&[(0, [0.0, 0.0, 0.0, 1.0]), (10, [0.0, 0.0, 0.0, 1.0])]),
    )
}

fn archive_with_clips(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut entries = npc_entries();
    entries.push(TestEntry::stored(
        "Data\\Animations\\pu\\male\\npc\\own.caf",
        clip("own.caf").caf(),
    ));
    entries.push(TestEntry::stored(
        "Data\\Animations\\Characters\\Human\\male_v7\\walk.caf",
        clip("walk.caf").caf(),
    ));
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn written(extra: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = archive_with_clips(dir.path())?;
    let out = dir.path().join("npc.glb");
    let run = glb(&archive, &out, extra, true)?;
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8(run.stderr)?
    );
    let json: serde_json::Value = serde_json::from_slice(&run.stdout)?;
    let mut names: Vec<String> = json
        .get("animations")
        .and_then(serde_json::Value::as_array)
        .ok_or("animations")?
        .iter()
        .filter_map(|animation| animation.get("name")?.as_str().map(str::to_owned))
        .collect();
    names.sort();
    Ok(names)
}

#[test]
fn an_animation_of_the_characters_own_folder_is_found() -> TestResult {
    assert_eq!(written(&["--anim", "own"])?, ["own"]);
    Ok(())
}

#[test]
fn an_animation_the_own_folder_lacks_is_found_in_the_folders_of_the_body() -> TestResult {
    assert_eq!(written(&["--anim", "walk"])?, ["walk"]);
    Ok(())
}

#[test]
fn all_animations_are_those_of_the_own_folder_only() -> TestResult {
    assert_eq!(written(&["--anim", "all"])?, ["own"]);
    Ok(())
}

#[test]
fn an_animation_folder_replaces_the_own_folder() -> TestResult {
    let names = written(&[
        "--anim",
        "all",
        "--anim-folder",
        "Animations/Characters/Human/male_v7",
    ])?;
    assert_eq!(names, ["walk"]);
    Ok(())
}

fn rigged_archive(path: &Path) -> Result<(), Box<dyn Error>> {
    let rig = head_rig(HEAD_VERTICES, MALE_RIG_HASH).build();
    let folder = MALE_HEAD_FOLDER.replace('/', "\\");
    let mut entries = npc_entries();
    for part in ["head", "teeth"] {
        entries.push(TestEntry::stored(
            format!("Data\\{folder}\\{MALE_RIG}_{part}.dna"),
            rig.clone(),
        ));
    }
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(path)?;
    Ok(())
}

fn hero_chf(path: &Path) -> Result<(), Box<dyn Error>> {
    let blend = |part: usize| {
        if part == 3 {
            vec![(65535, 2), (0, 0)]
        } else {
            vec![(0, 0), (0, 0)]
        }
    };
    let chf = TestChf {
        dna_hashes: Some((MALE_RIG_HASH, 0)),
        face_parts: (0..HEAD_REGIONS).map(blend).collect(),
        ..TestChf::v8()
    };
    std::fs::write(path, chf.to_bytes())?;
    Ok(())
}

fn package_heads(
    archive: &Path,
    out: &Path,
    extra: &[&str],
) -> Result<Vec<String>, Box<dyn Error>> {
    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["character", "package"])
        .arg(archive)
        .arg("--out")
        .arg(out)
        .args(["--loadout", NPC_CLASS, "--materials", "colours"])
        .args(extra)
        .output()?;
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "{}",
        String::from_utf8(output.stderr)?
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(json
        .get("parts")
        .and_then(serde_json::Value::as_array)
        .ok_or("parts")?
        .iter()
        .filter(|part| part.get("name").is_some_and(|name| name == "head"))
        .filter_map(|part| part.get("file")?.as_str().map(str::to_owned))
        .collect())
}

#[test]
fn a_chf_beside_a_loadout_drops_the_head_of_the_loadout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    rigged_archive(&path)?;
    let chf_path = dir.path().join("hero.chf");
    hero_chf(&chf_path)?;
    let heads =
        |extra: &[&str]| package_heads(&path, &dir.path().join(format!("p{}", extra.len())), extra);

    let own = heads(&[])?;
    assert!(
        own.iter().all(|file| file.contains("npc_test_t1_head")),
        "{own:?}"
    );
    assert_eq!(own.len(), 1);
    let with_chf = heads(&["--chf", chf_path.to_str().ok_or("path")?])?;
    assert_eq!(with_chf.len(), 1, "{with_chf:?}");
    assert!(
        !with_chf[0].contains("npc_test_t1_head") && with_chf[0].contains(MALE_RIG),
        "{with_chf:?}"
    );
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
