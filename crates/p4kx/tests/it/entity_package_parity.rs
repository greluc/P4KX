// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The node transforms of a package are those `p4kx entity glb` writes.

use std::error::Error;
use std::path::Path;

use p4kx_testkit::{assembly_entries, check_glb};
use serde_json::Value;

use crate::entity_package::{manifest, package, write_archive};

type TestResult = Result<(), Box<dyn Error>>;

fn numbers(node: &Value, key: &str) -> Result<Vec<f64>, Box<dyn Error>> {
    Ok(node[key]
        .as_array()
        .ok_or_else(|| key.to_owned())?
        .iter()
        .filter_map(Value::as_f64)
        .collect())
}

fn close(actual: &[f64], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - f64::from(*e)).abs() < 1e-5)
}

fn compare_with_glb(manifest: &Value, glb: &Path) -> Result<usize, Box<dyn Error>> {
    let checked = check_glb(&std::fs::read(glb)?)?;
    let mut compared = 0;
    for node in manifest["nodes"].as_array().ok_or("nodes")? {
        let name = node["name"].as_str().ok_or("name")?;
        let reference = checked
            .document
            .nodes()
            .find(|candidate| candidate.name() == Some(name))
            .ok_or_else(|| format!("no node {name}"))?;
        let (translation, rotation, scale) = reference.transform().decomposed();
        for (key, expected) in [
            ("translation", translation.to_vec()),
            ("rotation", rotation.to_vec()),
            ("scale", scale.to_vec()),
        ] {
            let actual = numbers(node, key)?;
            assert!(
                close(&actual, &expected),
                "{name} {key}: {actual:?} against {expected:?}"
            );
        }
        compared += 1;
    }
    Ok(compared)
}

#[test]
fn node_transforms_composed_with_the_mesh_root_are_those_of_entity_glb() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), assembly_entries())?;
    let out = dir.path().join("ship");
    let glb = dir.path().join("ship.glb");

    let packaged = package(&archive, "TEST_Ship", &out, &[], false)?;
    let exported = p4kx_testkit::p4kx()?
        .args(["entity", "glb"])
        .arg(&archive)
        .arg("TEST_Ship")
        .arg("--out")
        .arg(&glb)
        .output()?;

    assert_eq!(packaged.code, Some(0), "{}", packaged.stderr);
    assert_eq!(exported.status.code(), Some(0), "{exported:?}");
    let manifest = manifest(&out)?;
    assert_eq!(
        compare_with_glb(&manifest, &glb)?,
        6,
        "the root and the five parts of the ship"
    );
    let mount = manifest["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node["name"] == "hardpoint_weapon_nose/TEST_Mount")
        })
        .ok_or("mount")?;
    let turned = numbers(mount, "rotation")?;
    assert!(
        turned.iter().take(3).any(|axis| axis.abs() > 0.1),
        "the mount's port is turned, so a real rotation is compared: {turned:?}"
    );
    for entry in manifest["meshes"].as_array().ok_or("meshes")? {
        let file = entry["file"].as_str().ok_or("file")?;
        let mesh = check_glb(&std::fs::read(out.join(file))?)?;
        let root = mesh
            .document
            .scenes()
            .next()
            .and_then(|scene| scene.nodes().next())
            .ok_or("root")?;
        let (translation, rotation, scale) = root.transform().decomposed();
        assert_eq!(translation, [0.0; 3], "{file}");
        assert_eq!(rotation, [0.0, 0.0, 0.0, 1.0], "{file}");
        assert_eq!(scale, [1.0; 3], "{file}");
    }
    Ok(())
}
