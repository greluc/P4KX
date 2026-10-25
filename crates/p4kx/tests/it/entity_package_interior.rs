// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx entity package --interior` over the synthetic ship with a lit room: the layout, the
//! report and the agreement with `p4kx entity glb --interior`.

use std::error::Error;

use p4kx_testkit::{ENTITY_LIT_ROOM_CLASS, check_glb, entity_lit_room_entries};
use serde_json::Value;

use crate::entity_package::{manifest, package, write_archive};

type TestResult = Result<(), Box<dyn Error>>;

fn nodes_of_kind<'a>(manifest: &'a Value, kind: &str) -> Result<Vec<&'a Value>, Box<dyn Error>> {
    Ok(manifest["nodes"]
        .as_array()
        .ok_or("nodes")?
        .iter()
        .filter(|node| node["kind"] == kind)
        .collect())
}

#[test]
fn the_interior_option_adds_containers_objects_and_lights_and_the_schema_accepts_them() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), entity_lit_room_entries())?;
    let out = dir.path().join("ship");

    let run = package(
        &archive,
        ENTITY_LIT_ROOM_CLASS,
        &out,
        &["--interior"],
        false,
    )?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout
            .contains("Interior: 3 object containers, 2 objects, 5 lights"),
        "{}",
        run.stdout
    );
    let manifest = manifest(&out)?;
    let validator = jsonschema::draft202012::new(&p4kx_blender::manifest_schema())?;
    let violations: Vec<String> = validator
        .iter_errors(&manifest)
        .take(5)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(nodes_of_kind(&manifest, "light")?.len(), 5);
    assert_eq!(nodes_of_kind(&manifest, "object")?.len(), 2);
    assert_eq!(nodes_of_kind(&manifest, "container")?.len(), 3);
    for mesh in manifest["meshes"].as_array().ok_or("meshes")? {
        let file = mesh["file"].as_str().ok_or("file")?;
        check_glb(&std::fs::read(out.join(file))?)?;
    }
    Ok(())
}

#[test]
fn without_the_option_the_package_is_the_hull_alone() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), entity_lit_room_entries())?;
    let out = dir.path().join("ship");

    let run = package(&archive, ENTITY_LIT_ROOM_CLASS, &out, &[], true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(report["nodes"], 1);
    assert_eq!(report["lights"], 0);
    assert_eq!(report["containers"], 0);
    Ok(())
}

#[test]
fn the_json_report_counts_the_interior() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), entity_lit_room_entries())?;
    let out = dir.path().join("ship");

    let run = package(&archive, ENTITY_LIT_ROOM_CLASS, &out, &["--interior"], true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let report: Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(report["lights"], 5);
    assert_eq!(report["objects"], 2);
    assert_eq!(report["containers"], 3);
    Ok(())
}

#[test]
fn the_lights_of_the_package_are_those_of_entity_glb() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), entity_lit_room_entries())?;
    let out = dir.path().join("ship");
    let glb = dir.path().join("ship.glb");
    let run = package(
        &archive,
        ENTITY_LIT_ROOM_CLASS,
        &out,
        &["--interior"],
        false,
    )?;
    let exported = p4kx_testkit::p4kx()?
        .args(["entity", "glb"])
        .arg(&archive)
        .arg(ENTITY_LIT_ROOM_CLASS)
        .arg("--out")
        .arg(&glb)
        .arg("--interior")
        .output()?;
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(exported.status.code(), Some(0), "{exported:?}");

    let checked = check_glb(&std::fs::read(&glb)?)?;
    let manifest = manifest(&out)?;
    for node in nodes_of_kind(&manifest, "light")? {
        let name = node["name"].as_str().ok_or("name")?;
        let reference = checked
            .document
            .nodes()
            .find(|candidate| candidate.name() == Some(name))
            .ok_or_else(|| format!("no node {name}"))?;
        let (translation, rotation, _) = reference.transform().decomposed();
        for (key, expected) in [
            ("translation", translation.to_vec()),
            ("rotation", rotation.to_vec()),
        ] {
            let actual: Vec<f64> = node[key]
                .as_array()
                .ok_or("array")?
                .iter()
                .filter_map(Value::as_f64)
                .collect();
            assert!(
                actual
                    .iter()
                    .zip(&expected)
                    .all(|(a, e)| (a - f64::from(*e)).abs() < 1e-5),
                "{name} {key}: {actual:?} against {expected:?}"
            );
        }
    }
    assert_eq!(
        checked.document.lights().map_or(0, Iterator::count),
        5,
        "the GLB has the same number of lights"
    );
    Ok(())
}
