// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Layer files that name each other, leave the `Data` folder or chain deeper than the writer
//! follows.

use p4kx_blender::{IssueOrigin, Manifest, MaterialDescription, PackageInfo, write_package};
use p4kx_extraction::Archive;
use p4kx_testkit::layer_chain_entries;

use crate::support::{TestResult, assemble, fixture, glb_options, never};

fn write() -> Result<Manifest, Box<dyn std::error::Error>> {
    let fixture = fixture(layer_chain_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Chain")?;
    let models = p4kx_assembly::ArchiveModels::new(&archive);
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("ship");
    write_package(
        &assembly,
        &models,
        &models.textures(),
        &glb_options(0),
        &PackageInfo::default(),
        &out,
        &never,
    )?;
    Ok(serde_json::from_str(&std::fs::read_to_string(
        out.join("scene.json"),
    )?)?)
}

fn named<'a>(manifest: &'a Manifest, name: &str) -> Result<&'a MaterialDescription, String> {
    manifest
        .materials
        .iter()
        .find(|material| material.name == name)
        .ok_or_else(|| format!("no material {name}"))
}

fn next<'a>(
    manifest: &'a Manifest,
    material: &MaterialDescription,
) -> Result<Option<&'a MaterialDescription>, String> {
    let Some(id) = material
        .layers
        .first()
        .and_then(|layer| layer.material.as_deref())
    else {
        return Ok(None);
    };
    manifest
        .materials
        .iter()
        .find(|found| found.id == id)
        .map(Some)
        .ok_or_else(|| format!("no material {id}"))
}

#[test]
fn two_layer_files_that_name_each_other_refer_to_each_other() -> TestResult {
    let manifest = write()?;

    let cycle = named(&manifest, "cycle")?;
    let a = next(&manifest, cycle)?.ok_or("cyc_a")?;
    let b = next(&manifest, a)?.ok_or("cyc_b")?;
    let again = next(&manifest, b)?.ok_or("cyc_a again")?;
    assert_eq!(a.name, "cyc_a");
    assert_eq!(b.name, "cyc_b");
    assert_eq!(again.id, a.id, "the cycle closes on the first description");
    assert!(manifest.issues.iter().all(|issue| issue.name != "cyc_a"));
    Ok(())
}

#[test]
fn a_layer_path_that_leaves_the_data_folder_is_an_issue() -> TestResult {
    let manifest = write()?;

    let escape = named(&manifest, "escape")?;
    assert!(escape.layers.iter().all(|layer| layer.material.is_none()));
    let issue = manifest
        .issues
        .iter()
        .find(|issue| issue.path == "../outside.mtl")
        .ok_or("issue")?;
    assert_eq!(issue.origin, IssueOrigin::Material);
    assert_eq!(issue.kind, "layer_material");
    assert_eq!(issue.name, "escape");
    Ok(())
}

#[test]
fn a_material_is_followed_again_when_a_shallower_reach_lets_its_layers_go_further() -> TestResult {
    let manifest = write()?;

    let cycle = named(&manifest, "cycle")?;
    let layer_of = |number: usize| -> Result<&MaterialDescription, String> {
        let id = cycle
            .layers
            .get(number)
            .and_then(|layer| layer.material.as_deref())
            .ok_or_else(|| format!("no material of layer {number}"))?;
        manifest
            .materials
            .iter()
            .find(|found| found.id == id)
            .ok_or_else(|| format!("no material {id}"))
    };
    let short = layer_of(2)?;
    assert_eq!(short.name, "late_4");
    let mut last = short;
    for name in ["late_5", "late_6", "late_7", "late_8"] {
        last = next(&manifest, last)?.ok_or_else(|| format!("{name} follows"))?;
        assert_eq!(last.name, name);
    }
    assert!(next(&manifest, last)?.is_none());
    assert!(
        manifest.issues.iter().all(|issue| issue.name != "late_7"),
        "late_7 is followed, so no issue says it was not"
    );

    let mut reached = layer_of(1)?;
    while let Some(layer) = next(&manifest, reached)? {
        reached = layer;
    }
    assert_eq!(reached.id, last.id, "the long chain shares the description");
    Ok(())
}

#[test]
fn layer_chains_are_followed_eight_files_deep_and_the_cut_is_an_issue() -> TestResult {
    let manifest = write()?;

    let mut count = 0;
    let mut current = named(&manifest, "deep")?;
    while let Some(layer) = next(&manifest, current)? {
        count += 1;
        current = layer;
    }
    assert_eq!(count, 8);
    assert_eq!(current.name, "chain_07");
    let issue = manifest
        .issues
        .iter()
        .find(|issue| issue.name == "chain_07")
        .ok_or("issue")?;
    assert_eq!(issue.kind, "layer_material");
    assert!(
        !current.layers.is_empty(),
        "the layer stays, without a material"
    );
    Ok(())
}
