// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The material descriptions of `p4kx entity package` over the synthetic layered ship.

use std::error::Error;

use p4kx_testkit::layered_entries;

use crate::entity_package::{manifest, package, write_archive};

type TestResult = Result<(), Box<dyn Error>>;

fn pretty(manifest: &serde_json::Value, field: &str) -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string_pretty(&manifest[field])?)
}

#[test]
fn the_package_describes_every_layered_material() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), layered_entries())?;
    let out = dir.path().join("layered");

    let run = package(&archive, "TEST_Layered", &out, &[], false)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let manifest = manifest(&out)?;
    let validator = jsonschema::draft202012::new(&p4kx_blender::manifest_schema())?;
    assert!(validator.is_valid(&manifest));
    insta::assert_snapshot!("materials", pretty(&manifest, "materials")?);
    insta::assert_snapshot!("palettes", pretty(&manifest, "palettes")?);
    insta::assert_snapshot!(
        "mesh_materials",
        pretty(&manifest["meshes"][0], "materials")?
    );
    Ok(())
}

#[test]
fn the_summary_counts_the_materials_the_layers_and_the_palettes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path(), layered_entries())?;
    let out = dir.path().join("layered");

    let run = package(&archive, "TEST_Layered", &out, &[], true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let summary: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(summary["materials"], 7);
    assert_eq!(summary["palettes"], 1);
    Ok(())
}
