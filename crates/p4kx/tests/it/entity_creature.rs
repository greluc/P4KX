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

/// A creature whose character definition names the skin `body.skin`, whose `.skinm` companion is
/// `skinm` or absent.
fn creature_with_skin(skinm: Option<Vec<u8>>) -> Vec<TestEntry> {
    let skin = TestModel::two_bone_skin();
    let mut entries = creature("Objects/Test/creature.cdf");
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\creature.cdf",
        DEFINITION.as_bytes().to_vec(),
    ));
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\body.skin",
        skin.cgf(),
    ));
    entries.extend(skinm.map(|bytes| TestEntry::stored("Data\\Objects\\Test\\body.skinm", bytes)));
    entries
}

fn with_body() -> Vec<TestEntry> {
    creature_with_skin(Some(TestModel::two_bone_skin().cgfm()))
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
    let archive = write_archive(dir.path(), with_body())?;
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
    let archive = write_archive(dir.path(), with_body())?;
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

/// The geometry files of a creature (a skeleton) and of a ship-like hull.
const ROOT_GEOMETRIES: [&str; 2] = ["Objects/Test/rig.chr", "Objects/Test/hull.cga"];

/// Runs `entity glb` and `entity package` on an archive and checks that both fail with `code`,
/// name `needle` on stderr and leave nothing in `dir` but the archive.
fn assert_both_fail(dir: &Path, archive: &Path, code: i32, needle: &str) -> TestResult {
    let run = glb(archive, &dir.join("out.glb"), false)?;
    assert_eq!(run.code, Some(code), "{}", run.stderr);
    assert!(run.stderr.contains(needle), "{}", run.stderr);
    let run = package(archive, "TEST_Creature", &dir.join("package"), &[], false)?;
    assert_eq!(run.code, Some(code), "{}", run.stderr);
    assert!(run.stderr.contains(needle), "{}", run.stderr);
    let mut files: Vec<String> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?;
    files.sort();
    assert_eq!(files, ["Data.p4k"], "no output, no partial file");
    Ok(())
}

#[test]
fn a_root_geometry_that_is_missing_fails_with_exit_code_2_naming_the_file() -> TestResult {
    for geometry in ROOT_GEOMETRIES {
        let dir = tempfile::tempdir()?;
        let archive = write_archive(dir.path(), creature(geometry))?;

        assert_both_fail(
            dir.path(),
            &archive,
            2,
            &format!("{geometry} does not exist"),
        )?;
    }
    Ok(())
}

#[test]
fn a_root_geometry_that_cannot_be_read_fails_with_exit_code_3_naming_the_file() -> TestResult {
    for geometry in ROOT_GEOMETRIES {
        let dir = tempfile::tempdir()?;
        let mut entries = creature(geometry);
        entries.push(TestEntry::stored(
            format!("Data\\{}", geometry.replace('/', "\\")),
            b"not a chunk file".to_vec(),
        ));
        let archive = write_archive(dir.path(), entries)?;

        assert_both_fail(dir.path(), &archive, 3, geometry)?;
    }
    Ok(())
}

#[test]
fn a_root_geometry_whose_companion_is_missing_fails_naming_the_companion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut entries = creature("Objects/Test/rig.chr");
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\rig.chr",
        TestModel::two_bone_skin().cgf(),
    ));
    let archive = write_archive(dir.path(), entries)?;

    assert_both_fail(dir.path(), &archive, 3, "rig.chrm")
}

#[test]
fn a_part_below_the_root_that_cannot_be_drawn_stays_an_issue() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), p4kx_testkit::assembly_entries())?;
    let out = dir.path().join("ship.glb");

    let output = p4kx_testkit::p4kx()?
        .args(["entity", "glb"])
        .arg(&archive)
        .arg("TEST_Ship")
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8(output.stdout)?.contains("skipped hardpoint_door"));
    check_glb(&std::fs::read(&out)?)?;
    Ok(())
}

#[test]
fn an_entity_whose_root_names_no_geometry_is_still_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let entity = TestEntity::new(0x1, "TEST_Creature", "entities/test_creature.xml");
    let database = entity_schema(DataForgeBuilder::new()).record(entity.record());
    let archive = write_archive(
        dir.path(),
        vec![TestEntry::zstd("Data\\Game2.dcb", database.build())],
    )?;
    let out = dir.path().join("empty.glb");

    let run = glb(&archive, &out, true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(
        issue_kinds(&report, "no_geometry"),
        [("no_geometry", false)]
    );
    check_glb(&std::fs::read(&out)?)?;

    let package_out = dir.path().join("package");
    let run = package(&archive, "TEST_Creature", &package_out, &[], false)?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(package_out.join("scene.json").is_file());
    Ok(())
}

#[test]
fn a_character_definition_that_names_no_model_fails_with_exit_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut entries = creature("Objects/Test/creature.cdf");
    entries.push(TestEntry::stored(
        "Data\\Objects\\Test\\creature.cdf",
        b"<CharacterDefinition/>".to_vec(),
    ));
    let archive = write_archive(dir.path(), entries)?;

    assert_both_fail(dir.path(), &archive, 3, "creature.cdf")
}

#[test]
fn a_skin_whose_skinm_companion_is_missing_fails_with_exit_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), creature_with_skin(None))?;

    assert_both_fail(dir.path(), &archive, 3, "body.skinm")
}

#[test]
fn a_skin_whose_skinm_companion_is_corrupt_fails_with_exit_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let corrupt = b"not a chunk file".to_vec();
    let archive = write_archive(dir.path(), creature_with_skin(Some(corrupt)))?;

    assert_both_fail(dir.path(), &archive, 3, "body.skinm")
}

#[test]
fn a_failed_json_run_prints_the_error_on_stderr_and_nothing_on_stdout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), creature("Objects/Test/rig.chr"))?;

    let run = glb(&archive, &dir.path().join("out.glb"), true)?;
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr.contains("Objects/Test/rig.chr does not exist"),
        "{}",
        run.stderr
    );

    let run = package(
        &archive,
        "TEST_Creature",
        &dir.path().join("package"),
        &[],
        true,
    )?;
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr.contains("Objects/Test/rig.chr does not exist"),
        "{}",
        run.stderr
    );
    Ok(())
}
