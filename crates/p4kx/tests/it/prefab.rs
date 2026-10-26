// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx prefab list` and `p4kx prefab dump` end to end over a synthetic nested library.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestPrefabLibrary, TestPrefabObject};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];
const QUARTER_Z: [f64; 4] = [
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
    std::f64::consts::FRAC_1_SQRT_2,
];

struct Run {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let rooms = TestPrefabLibrary::new("test/rooms")
        .prefab(
            "hall",
            vec![
                TestPrefabObject::brush("wall", "objects/test/wall.cgf", [1.0, 2.0, 3.0], IDENTITY),
                TestPrefabObject::nested(
                    "sub",
                    "test/rooms",
                    "closet",
                    [10.0, 0.0, 0.0],
                    QUARTER_Z,
                ),
                TestPrefabObject::nested("ghost", "test/rooms", "nothing", [0.0; 3], IDENTITY),
            ],
        )
        .prefab(
            "closet",
            vec![TestPrefabObject::brush(
                "shelf",
                "objects/test/shelf.cgf",
                [1.0, 0.0, 0.0],
                IDENTITY,
            )],
        );
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new().entry(rooms.entry()).write_to(&path)?;
    Ok(path)
}

fn run(args: &[&str], archive: &Path, json: bool) -> Result<Run, Box<dyn Error>> {
    let (action, rest) = args.split_first().ok_or("no action")?;
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    let output = command
        .arg("prefab")
        .arg(action)
        .arg(archive)
        .args(rest)
        .output()?;
    Ok(Run {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code(),
    })
}

#[test]
fn the_libraries_and_their_prefabs_are_listed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let libraries = run(&["list"], &archive, false)?;
    assert_eq!(libraries.code, Some(0), "{}", libraries.stderr);
    assert_eq!(libraries.stdout.trim(), "test/rooms");

    let prefabs = run(&["list", "TEST/Rooms"], &archive, true)?;
    assert_eq!(prefabs.code, Some(0), "{}", prefabs.stderr);
    let json: Value = serde_json::from_str(&prefabs.stdout)?;
    assert_eq!(json["library"], "test/rooms");
    assert_eq!(json["prefabs"][0]["name"], "hall");
    assert_eq!(json["prefabs"][0]["objects"], 3);
    assert_eq!(json["prefabs"][0]["nested"], 2);
    assert_eq!(json["prefabs"][1]["name"], "closet");

    let table = run(&["list", "test/rooms"], &archive, false)?;
    assert!(table.stdout.contains("hall"), "{}", table.stdout);
    Ok(())
}

#[test]
fn a_prefab_is_dumped_with_its_nested_prefabs_resolved() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let dump = run(&["dump", "test/rooms", "hall"], &archive, true)?;

    assert_eq!(dump.code, Some(0), "{}", dump.stderr);
    let json: Value = serde_json::from_str(&dump.stdout)?;
    assert_eq!(json["prefabs_entered"], 2);
    let objects = json["objects"].as_array().ok_or("no objects")?;
    assert_eq!(objects.len(), 3);
    let shelf = objects
        .iter()
        .find(|object| object["name"] == "shelf")
        .ok_or("no shelf")?;
    assert_eq!(shelf["container"], "sub");
    assert_eq!(shelf["model"], "objects/test/shelf.cgf");
    let position: Vec<f64> = shelf["position"]
        .as_array()
        .ok_or("no position")?
        .iter()
        .filter_map(Value::as_f64)
        .collect();
    assert!(
        position
            .iter()
            .zip([10.0, 1.0, 0.0])
            .all(|(a, e)| (a - e).abs() < 1e-4),
        "{position:?}"
    );
    assert_eq!(json["issues"][0]["kind"], "missing");

    let text = run(&["dump", "test/rooms", "HALL"], &archive, false)?;
    assert_eq!(text.code, Some(0), "{}", text.stderr);
    assert!(text.stdout.contains("sub/shelf"), "{}", text.stdout);
    assert!(text.stdout.contains("note ghost"), "{}", text.stdout);
    Ok(())
}

#[test]
fn an_unknown_library_or_prefab_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    for args in [
        &["list", "test/none"][..],
        &["dump", "test/none", "hall"],
        &["dump", "test/rooms", "none"],
    ] {
        let run = run(args, &archive, false)?;
        assert_eq!(run.code, Some(2), "{args:?}: {}", run.stderr);
        assert!(run.stdout.is_empty(), "{}", run.stdout);
    }
    Ok(())
}
