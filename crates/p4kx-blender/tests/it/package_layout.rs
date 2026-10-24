// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layout of a package written from the synthetic ship `TEST_Livery`: files, manifest,
//! schema, shared parts and textures.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use p4kx_blender::{
    BuildInfo, FORMAT, FORMAT_VERSION, Manifest, PackageInfo, manifest_schema, write_package,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{check_glb, livery_entries};

use crate::support::{TestResult, assemble, fixture, glb_options, names_in, never};

struct Written {
    _dir: tempfile::TempDir,
    out: PathBuf,
    manifest: Manifest,
    raw: serde_json::Value,
    summary: p4kx_blender::PackageSummary,
}

fn write(mip: u32) -> Result<Written, Box<dyn std::error::Error>> {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let models = p4kx_assembly::ArchiveModels::new(&archive);
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("ship");
    let info = PackageInfo::new(
        Some("LIVE".to_owned()),
        Some(BuildInfo {
            branch: "sc-alpha-4.0.0".to_owned(),
            version: "4.0.0".to_owned(),
            public_patch: None,
            change_number: "1".to_owned(),
            build_id: "2".to_owned(),
        }),
    );
    let summary = write_package(
        &assembly,
        &models.textures(),
        &glb_options(mip),
        &info,
        &out,
        &never,
    )?;
    let text = std::fs::read_to_string(out.join("scene.json"))?;
    Ok(Written {
        manifest: serde_json::from_str(&text)?,
        raw: serde_json::from_str(&text)?,
        _dir: dir,
        out,
        summary,
    })
}

fn read(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(std::fs::read(path)?)
}

#[test]
fn the_package_is_a_folder_of_a_manifest_parts_and_textures() -> TestResult {
    let written = write(2)?;

    assert_eq!(
        names_in(&written.out)?,
        ["parts", "scene.json", "textures"],
        "nothing else is in the folder"
    );
    assert_eq!(
        names_in(&written.out.join("parts"))?,
        ["p0001.glb", "p0002.glb", "p0003.glb", "p0004.glb"]
    );
    assert_eq!(names_in(&written.out.join("textures"))?, ["t0001.png"]);
    assert_eq!(written.summary.files, 6);
    let on_disk: u64 = [
        written.out.join("scene.json"),
        written.out.join("parts/p0001.glb"),
        written.out.join("parts/p0002.glb"),
        written.out.join("parts/p0003.glb"),
        written.out.join("parts/p0004.glb"),
        written.out.join("textures/t0001.png"),
    ]
    .iter()
    .map(|path| std::fs::metadata(path).map(|metadata| metadata.len()))
    .sum::<Result<u64, _>>()?;
    assert_eq!(
        written.summary.bytes, on_disk,
        "the summary counts every file"
    );
    Ok(())
}

#[test]
fn the_manifest_states_version_source_and_axes() -> TestResult {
    let written = write(2)?;
    let manifest = &written.manifest;

    assert_eq!(manifest.format, FORMAT);
    assert_eq!(manifest.version, FORMAT_VERSION);
    assert_eq!(manifest.generator.name, "p4kx");
    assert_eq!(manifest.source.channel.as_deref(), Some("LIVE"));
    assert_eq!(
        manifest
            .source
            .build
            .as_ref()
            .map(|build| build.version.as_str()),
        Some("4.0.0")
    );
    assert_eq!(manifest.source.entity.class_name, "TEST_Livery");
    assert_eq!(manifest.coordinates.basis, "gltf_y_up");
    assert_eq!(manifest.coordinates.unit, "metre");
    assert_eq!(manifest.options.texture_mip, 2);
    Ok(())
}

#[test]
fn the_manifest_matches_its_schema() -> TestResult {
    let written = write(0)?;
    let validator = jsonschema::draft202012::new(&manifest_schema())?;

    let violations: Vec<String> = validator
        .iter_errors(&written.raw)
        .take(5)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();

    assert!(violations.is_empty(), "{violations:?}");
    Ok(())
}

#[test]
fn nodes_keep_the_hierarchy_and_names_of_the_assembly() -> TestResult {
    let written = write(0)?;
    let nodes = &written.manifest.nodes;

    let names: Vec<&str> = nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "TEST_Livery",
            "hardpoint_painted/TEST_Painted",
            "hardpoint_twin/TEST_Painted",
            "hardpoint_default/TEST_Default",
            "hardpoint_lost/TEST_Lost",
            "hardpoint_child/TEST_Child",
            "hardpoint_plain/TEST_Plain",
        ]
    );
    assert_eq!(nodes.first().and_then(|node| node.parent), None);
    assert!(
        nodes.iter().skip(1).all(|node| node.parent == Some(0)),
        "every part hangs below the ship"
    );
    assert_eq!(
        nodes.get(1).map(|node| node.path.as_str()),
        Some("hardpoint_painted")
    );
    assert!(
        nodes.iter().all(
            |node| node.scale.iter().all(|s| (s.abs() - 1.0).abs() < 1e-5)
                && (node.rotation.iter().map(|q| q * q).sum::<f32>() - 1.0).abs() < 1e-4
        ),
        "unit scale and unit quaternions"
    );
    Ok(())
}

#[test]
fn parts_of_one_model_library_and_palette_are_written_once_and_instanced() -> TestResult {
    let written = write(0)?;
    let manifest = &written.manifest;

    let part_of = |index: usize| {
        manifest
            .nodes
            .get(index)
            .and_then(|node| node.part.as_deref())
    };
    assert_eq!(manifest.parts.len(), 4, "ship, painted, default, plain");
    assert_eq!(part_of(1), part_of(2), "the two painted guns share a part");
    assert_eq!(part_of(4), part_of(5), "lost and child fall back to none");
    assert_eq!(part_of(4), part_of(6), "and so does the plain one");
    let distinct: BTreeSet<_> = (0..7).filter_map(part_of).collect();
    assert_eq!(distinct.len(), 4);
    let instances: Vec<usize> = manifest.parts.iter().map(|part| part.instances).collect();
    assert_eq!(instances, [1, 2, 1, 3]);
    assert_eq!(written.summary.drawn_nodes, 7);
    assert_eq!(written.summary.shared_parts, 3);
    let palettes: Vec<Option<&str>> = manifest
        .parts
        .iter()
        .map(|part| part.palette.as_deref())
        .collect();
    assert_eq!(
        palettes,
        [Some("p_ship"), Some("p_gun"), Some("p_default"), None]
    );
    Ok(())
}

#[test]
fn every_part_is_a_valid_glb_with_one_model_below_an_identity_root() -> TestResult {
    let written = write(0)?;

    for part in &written.manifest.parts {
        let bytes = read(&written.out.join(&part.file))?;
        let checked = check_glb(&bytes)?;
        assert_eq!(part.bytes, u64::try_from(bytes.len())?, "{}", part.id);
        assert_eq!(checked.document.images().len(), 0, "textures stay outside");
        assert_eq!(checked.document.scenes().len(), 1);
        assert_eq!(checked.document.meshes().len(), 1, "{}", part.id);
        assert!(part.triangles > 0 && part.vertices > 0);
        assert!(part.geometry.is_some(), "{}", part.id);
    }
    Ok(())
}

#[test]
fn textures_are_written_once_at_the_chosen_mip_and_named_by_the_materials() -> TestResult {
    let written = write(2)?;
    let manifest = &written.manifest;

    assert_eq!(manifest.textures.len(), 1);
    let texture = manifest.textures.first().ok_or("texture")?;
    assert_eq!(texture.file, "textures/t0001.png");
    assert_eq!(texture.source, "Objects/Test/livery_diff.dds");
    assert_eq!((texture.format.as_str(), texture.bit_depth), ("png", 8));
    assert_eq!((texture.width, texture.height), (2, 2), "8 by 8 at mip 2");
    let decoder = png::Decoder::new(std::io::Cursor::new(read(
        &written.out.join(&texture.file),
    )?));
    let info = decoder.read_info()?;
    assert_eq!(
        (info.info().width, info.info().height),
        (2, 2),
        "the file is what the manifest says"
    );
    assert_eq!(written.summary.textures, 1);
    assert_eq!(written.summary.texture_pixels, 4);
    let used: Vec<Option<&str>> = manifest
        .parts
        .iter()
        .flat_map(|part| &part.materials)
        .map(|material| material.textures.diffuse.as_deref())
        .collect();
    assert_eq!(used.len(), 8, "caps and sides of four parts");
    assert!(used.iter().all(|id| *id == Some("t0001")), "{used:?}");
    Ok(())
}

#[test]
fn what_the_assembly_could_not_resolve_is_listed_as_issues() -> TestResult {
    let written = write(0)?;

    let kinds: Vec<&str> = written
        .manifest
        .issues
        .iter()
        .map(|issue| issue.kind.as_str())
        .collect();
    assert_eq!(kinds, ["palette", "palette"], "lost and child");
    assert!(
        written
            .manifest
            .issues
            .iter()
            .all(|issue| issue.origin == "assembly" && !issue.skipped)
    );
    Ok(())
}
