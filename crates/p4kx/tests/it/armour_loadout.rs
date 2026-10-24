// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx armour glb --loadout` end to end over the synthetic characters of the testkit.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, armour_entries, armour_xml_loadout, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    armour_entries()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(path)
}

fn glb(archive: &Path, out: &Path, args: &[&str], json: bool) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    command
        .args(["armour", "glb"])
        .arg(archive)
        .arg("--out")
        .arg(out)
        .args(args);
    let output = command.output()?;
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code(),
    })
}

fn names(value: &serde_json::Value) -> Result<Vec<&str>, Box<dyn Error>> {
    Ok(value["parts"]
        .as_array()
        .ok_or("parts")?
        .iter()
        .filter_map(|part| part["name"].as_str())
        .collect())
}

#[test]
fn the_armour_and_clothing_of_a_character_loadout_are_worn() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("soldier.glb");

    let run = glb(&archive, &out, &["--loadout", "TEST_Soldier"], true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(
        names(&value)?,
        [
            "body",
            "torso/TEST_Torso",
            "helmet/TEST_Helmet",
            "legs/TEST_Pants"
        ],
        "the pistol is no piece"
    );
    assert_eq!(value["loadout"]["source"], "entity");
    assert_eq!(value["loadout"]["class_name"], "TEST_Soldier");
    assert_eq!(value["pieces"], 4, "three worn and the one that is lost");
    assert_eq!(value["skipped"], 1);
    let lost: Vec<&str> = value["issues"]
        .as_array()
        .ok_or("issues")?
        .iter()
        .filter(|issue| issue["kind"] == "piece_not_found")
        .filter_map(|issue| issue["name"].as_str())
        .collect();
    assert_eq!(lost, ["lost/TEST_Gone"]);
    check_glb(&std::fs::read(&out)?)?;
    Ok(())
}

#[test]
fn unresolved_pieces_and_pieces_that_do_not_fit_the_body_are_listed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("soldier.glb");

    let run = glb(
        &archive,
        &out,
        &["--loadout", "TEST_Soldier", "--body", "female"],
        false,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    for expected in [
        "loadout TEST_Soldier (entity)",
        "part torso/TEST_Torso: Objects/Characters/Human/female_v2/armor/test/f_suit.skin",
        "skipped lost/TEST_Gone: the piece cannot be resolved: no Entity has the class name TEST_Gone",
        "skipped helmet/TEST_Helmet: the Entity has no geometry for the female body",
    ] {
        assert!(run.stdout.contains(expected), "{expected}: {}", run.stdout);
    }
    Ok(())
}

#[test]
fn a_piece_replaces_the_loadout_piece_of_its_item_type_and_other_pieces_are_added() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("soldier.glb");

    let run = glb(
        &archive,
        &out,
        &[
            "--loadout",
            "TEST_Soldier",
            "--piece",
            "TEST_Plate",
            "--piece",
            "TEST_Suit",
        ],
        true,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(
        names(&value)?,
        [
            "body",
            "helmet/TEST_Helmet",
            "legs/TEST_Pants",
            "TEST_Plate",
            "TEST_Suit"
        ]
    );
    assert_eq!(
        value["replaced"],
        serde_json::json!([{
            "name": "torso/TEST_Torso",
            "by": "TEST_Plate",
            "item_type": "Char_Armor_Torso"
        }])
    );

    let text = glb(
        &archive,
        &out,
        &["--loadout", "TEST_Soldier", "--piece", "TEST_Plate"],
        false,
    )?;
    assert!(
        text.stdout
            .contains("replaced torso/TEST_Torso by TEST_Plate (Char_Armor_Torso)"),
        "{}",
        text.stdout
    );
    Ok(())
}

#[test]
fn an_xml_loadout_is_taken_from_an_entity_the_archive_or_a_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("grunt.glb");
    let (path, bytes) = armour_xml_loadout();
    let local = dir.path().join("grunt.xml");
    std::fs::write(&local, bytes)?;
    let local = local.to_str().ok_or("path")?;

    for (text, source) in [
        ("TEST_Grunt", "entity"),
        (path.as_str(), "archive"),
        (local, "file"),
    ] {
        let run = glb(&archive, &out, &["--loadout", text], true)?;
        assert_eq!(run.code, Some(0), "{text}: {}", run.stderr);
        let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
        assert_eq!(value["loadout"]["source"], source, "{text}");
        assert_eq!(
            names(&value)?,
            ["body", "torso/TEST_Torso", "helmet/TEST_Helmet"],
            "{text}"
        );
        assert_eq!(value["skipped"], 0, "{text}");
    }
    Ok(())
}

#[test]
fn a_loadout_that_names_nothing_readable_is_invalid_input_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("none.glb");

    for text in ["TEST_Nobody", "Scripts/Loadouts/None.xml"] {
        let run = glb(&archive, &out, &["--loadout", text], false)?;
        assert_eq!(run.code, Some(2), "{text}: {}", run.stderr);
        assert!(run.stderr.contains(text), "{text}: {}", run.stderr);
        assert!(!out.exists(), "no output");
    }
    Ok(())
}
