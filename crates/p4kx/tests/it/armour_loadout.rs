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

fn local_loadout(
    dir: &Path,
    items: &[(&str, &str)],
    includes: &[&str],
) -> Result<String, Box<dyn Error>> {
    use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};
    let mut list = TestElement::new("Items");
    for (port, class) in items {
        list = list.child(
            TestElement::new("Item")
                .attribute("portName", *port)
                .attribute("itemName", *class),
        );
    }
    let mut root = TestElement::new("Loadout");
    if !includes.is_empty() {
        let mut loadouts = TestElement::new("Loadouts");
        for include in includes {
            loadouts = loadouts.child(TestElement::new("Loadout").attribute("loadout", *include));
        }
        root = root.child(loadouts);
    }
    let path = dir.join("local.xml");
    std::fs::write(
        &path,
        root.child(list)
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
    )?;
    Ok(path.to_str().ok_or("path")?.to_owned())
}

#[test]
fn a_file_on_disk_takes_its_includes_from_the_archive_and_names_itself_by_its_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("local.glb");
    let local = local_loadout(
        dir.path(),
        &[("pants", "TEST_Pants"), ("lost", "TEST_Gone")],
        &[
            p4kx_testkit::ARMOUR_XML_LOADOUT,
            "Scripts/Loadouts/Missing.xml",
        ],
    )?;

    let run = glb(&archive, &out, &["--loadout", &local], false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    for expected in [
        "part torso/TEST_Torso:",
        "part helmet/TEST_Helmet:",
        "part pants/TEST_Pants:",
        "skipped lost/TEST_Gone:",
        &format!("skipped {local}:"),
    ] {
        assert!(run.stdout.contains(expected), "{expected}: {}", run.stdout);
    }
    assert!(!run.stdout.contains("local:loadout"), "{}", run.stdout);
    assert!(!run.stderr.contains("local:loadout"), "{}", run.stderr);
    Ok(())
}

fn lost_names(value: &serde_json::Value) -> Result<Vec<&str>, Box<dyn Error>> {
    Ok(value["issues"]
        .as_array()
        .ok_or("issues")?
        .iter()
        .filter(|issue| issue["kind"] == "piece_not_found")
        .filter_map(|issue| issue["name"].as_str())
        .collect())
}

#[test]
fn a_worn_piece_below_another_worn_entity_is_worn_and_what_fails_there_is_listed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("ranger.glb");
    let run = glb(&archive, &out, &["--loadout", "TEST_Ranger"], true)?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(
        names(&value)?,
        [
            "body",
            "pack/TEST_Pack",
            "pouch/TEST_Pouch",
            "jacket/TEST_Jacket"
        ],
        "the pouch hangs on the backpack"
    );
    assert_eq!(
        lost_names(&value)?,
        ["spare/TEST_Missing"],
        "the missing optic below the rifle is no armour"
    );
    assert_eq!(value["skipped"], 1, "the jacket is worn, not skipped");
    assert_eq!(value["pieces"], 4);
    assert_eq!(
        kinds_of(&value, "jacket/TEST_Jacket")?,
        ["loadout_unreadable"]
    );
    check_glb(&std::fs::read(&out)?)?;
    Ok(())
}

fn kinds_of<'a>(value: &'a serde_json::Value, name: &str) -> Result<Vec<&'a str>, Box<dyn Error>> {
    Ok(value["issues"]
        .as_array()
        .ok_or("issues")?
        .iter()
        .filter(|issue| issue["name"] == name)
        .filter_map(|issue| issue["kind"].as_str())
        .collect())
}

#[test]
fn a_piece_that_replaces_a_worn_entity_takes_its_loadout_issues_with_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("ranger.glb");
    let args = ["--loadout", "TEST_Ranger", "--piece", "TEST_Cuirass"];
    let run = glb(&archive, &out, &args, true)?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert!(
        kinds_of(&value, "jacket/TEST_Jacket")?.is_empty(),
        "{value}"
    );
    assert_eq!(value["replaced"][0]["name"], "jacket/TEST_Jacket");
    Ok(())
}

#[test]
fn a_local_loadout_file_over_the_cap_is_invalid_input_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("big.glb");
    let big = dir.path().join("big.xml");
    std::fs::File::create(&big)?.set_len(p4kx_assembly::MAX_XML_LOADOUT_LEN + 1)?;
    let big = big.to_str().ok_or("path")?;
    let run = glb(&archive, &out, &["--loadout", big], false)?;
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(run.stderr.contains("the limit of"), "{}", run.stderr);
    assert!(!out.exists(), "no output");
    Ok(())
}

#[test]
fn every_loadout_piece_of_the_type_of_a_piece_is_replaced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("two.glb");
    let local = local_loadout(
        dir.path(),
        &[
            ("torso", "TEST_Torso"),
            ("vest", "TEST_Plate"),
            ("legs", "TEST_Pants"),
        ],
        &[],
    )?;

    let run = glb(
        &archive,
        &out,
        &["--loadout", &local, "--piece", "TEST_Cuirass"],
        true,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(names(&value)?, ["body", "legs/TEST_Pants", "TEST_Cuirass"]);
    let replaced: Vec<&str> = value["replaced"]
        .as_array()
        .ok_or("replaced")?
        .iter()
        .filter_map(|each| each["name"].as_str())
        .collect();
    assert_eq!(replaced, ["torso/TEST_Torso", "vest/TEST_Plate"]);
    Ok(())
}
