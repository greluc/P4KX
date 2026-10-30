// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity glb` and `entity package` of an Entity whose geometry is a character file
//! without the files around it: the missing file is listed, never a panic or a partial file.

use std::error::Error;
use std::path::Path;

use p4kx_testkit::{DataForgeBuilder, TestEntity, TestEntry, TestModel, check_glb, entity_schema};
use serde_json::Value;

use crate::entity_package::{package, write_archive};

type TestResult = Result<(), Box<dyn Error>>;

const DEFINITION: &str = r#"<CharacterDefinition>
  <Model File="Objects\Test\rig_CHR.chr" LODRatio="40"/>
  <AttachmentList>
    <Attachment Type="CA_SKIN" AName="Body" Binding="Objects\Test\body.skin"/>
  </AttachmentList>
</CharacterDefinition>"#;

fn creature(geometry: &str) -> Vec<TestEntry> {
    let entity = TestEntity::new(0x1, "TEST_Creature", "entities/test_creature.xml")
        .geometry(geometry, None);
    let database = entity_schema(DataForgeBuilder::new()).record(entity.record());
    vec![TestEntry::zstd("Data\\Game2.dcb", database.build())]
}

fn with_body(mut entries: Vec<TestEntry>) -> Vec<TestEntry> {
    let skin = TestModel::two_bone_skin();
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\creature.cdf",
        DEFINITION.as_bytes().to_vec(),
    ));
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\body.skin",
        skin.cgf(),
    ));
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\body.skinm",
        skin.cgfm(),
    ));
    entries
}

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

fn glb(archive: &Path, out: &Path, json: bool) -> Result<Run, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    let output = command
        .args(["entity", "glb"])
        .arg(archive)
        .arg("TEST_Creature")
        .arg("--out")
        .arg(out)
        .output()?;
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code(),
    })
}

fn issue_kinds<'a>(report: &'a Value, prefix: &str) -> Vec<(&'a str, bool)> {
    report["issues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|issue| Some((issue["kind"].as_str()?, issue["skipped"].as_bool()?)))
        .filter(|(kind, _)| kind.starts_with(prefix))
        .collect()
}

#[test]
fn a_creature_without_its_skeleton_file_is_written_with_a_note() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), with_body(creature("Objects/Test/creature.cdf")))?;
    let out = dir.path().join("creature.glb");

    let run = glb(&archive, &out, false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout.contains(
            "note TEST_Creature: the skeleton Objects/Test/rig_CHR.chr the character definition \
             names does not exist"
        ),
        "{}",
        run.stdout
    );
    let checked = check_glb(&std::fs::read(&out)?)?;
    assert_eq!(checked.document.meshes().count(), 1);

    let json = glb(&archive, &out, true)?;
    let report: Value = serde_json::from_str(&json.stdout)?;
    assert_eq!(
        issue_kinds(&report, "skeleton"),
        [("skeleton_missing", false)]
    );
    Ok(())
}

#[test]
fn a_creature_without_its_skeleton_file_is_packaged_with_a_note() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), with_body(creature("Objects/Test/creature.cdf")))?;
    let out = dir.path().join("package");

    let run = package(&archive, "TEST_Creature", &out, &[], true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(
        issue_kinds(&report, "skeleton"),
        [("skeleton_missing", false)]
    );
    assert!(out.join("scene.json").is_file());
    Ok(())
}

#[test]
fn a_creature_whose_geometry_file_is_missing_is_a_valid_empty_scene_that_names_the_file()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), creature("Objects/Test/absent_skel.chr"))?;
    let out = dir.path().join("creature.glb");

    let run = glb(&archive, &out, false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout.contains(
            "skipped TEST_Creature: the geometry Objects/Test/absent_skel.chr does not exist"
        ),
        "{}",
        run.stdout
    );
    let checked = check_glb(&std::fs::read(&out)?)?;
    assert_eq!(checked.document.meshes().count(), 0);

    let packaged = dir.path().join("package");
    let run = package(&archive, "TEST_Creature", &packaged, &[], true)?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(
        issue_kinds(&report, "geometry"),
        [("geometry_missing", true)]
    );
    Ok(())
}

#[test]
fn a_creature_whose_geometry_companion_is_missing_names_the_companion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let skin = TestModel::two_bone_skin();
    let mut entries = creature("Objects/Test/rig.chr");
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\rig.chr",
        skin.cgf(),
    ));
    let archive = write_archive(dir.path(), entries)?;
    let out = dir.path().join("creature.glb");

    let run = glb(&archive, &out, false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("rig.chrm"), "{}", run.stdout);
    check_glb(&std::fs::read(&out)?)?;
    Ok(())
}

#[test]
fn a_damaged_creature_file_is_unreadable_and_leaves_no_partial_output() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut entries = creature("Objects/Test/rig.chr");
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\rig.chr",
        b"not a chunk file".to_vec(),
    ));
    let archive = write_archive(dir.path(), entries)?;
    let out = dir.path().join("creature.glb");

    let run = glb(&archive, &out, false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout.contains("skipped TEST_Creature"),
        "{}",
        run.stdout
    );
    check_glb(&std::fs::read(&out)?)?;
    let mut files: Vec<String> = std::fs::read_dir(dir.path())?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?;
    files.sort();
    assert_eq!(files, ["Data.p4k", "creature.glb"]);
    Ok(())
}
