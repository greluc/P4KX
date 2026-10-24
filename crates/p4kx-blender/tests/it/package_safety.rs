// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Where a package may be written, and what a failed or cancelled run leaves behind.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{ArchiveModels, Assembly};
use p4kx_blender::{PackageError, PackageInfo, write_package};
use p4kx_extraction::Archive;
use p4kx_gltf::{LoadedTexture, TextureLoadError, TextureLoader};
use p4kx_testkit::livery_entries;

use crate::support::{TestResult, assemble, fixture, glb_options, names_in, never};

struct Failing;

impl TextureLoader for Failing {
    fn load(&self, _path: &str, _mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        Err(TextureLoadError::Unreadable("broken on purpose".to_owned()))
    }
}

fn write(
    assembly: &Assembly,
    archive: &Archive,
    out: &Path,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<p4kx_blender::PackageSummary, PackageError> {
    let models = ArchiveModels::new(archive);
    write_package(
        assembly,
        &models.textures(),
        &glb_options(0),
        &PackageInfo::default(),
        out,
        cancelled,
    )
}

#[test]
fn an_earlier_package_is_replaced_and_nothing_else_is_left_beside_it() -> TestResult {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("ship");

    write(&assembly, &archive, &out, &never)?;
    std::fs::write(out.join("parts").join("stale.glb"), b"stale")?;
    write(&assembly, &archive, &out, &never)?;

    assert_eq!(
        names_in(dir.path())?,
        ["ship"],
        "no temporary folder remains"
    );
    assert!(!out.join("parts").join("stale.glb").exists());
    assert!(out.join("scene.json").is_file());
    Ok(())
}

#[test]
fn an_empty_folder_and_a_missing_parent_are_fine() -> TestResult {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let dir = tempfile::tempdir()?;
    let empty = dir.path().join("empty");
    std::fs::create_dir(&empty)?;
    let nested = dir.path().join("a").join("b").join("ship");

    write(&assembly, &archive, &empty, &never)?;
    write(&assembly, &archive, &nested, &never)?;

    assert!(empty.join("scene.json").is_file());
    assert!(nested.join("scene.json").is_file());
    Ok(())
}

#[test]
fn a_folder_with_other_files_or_a_file_is_not_replaced() -> TestResult {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let dir = tempfile::tempdir()?;
    let documents = dir.path().join("documents");
    std::fs::create_dir(&documents)?;
    std::fs::write(documents.join("thesis.txt"), b"precious")?;
    let file = dir.path().join("file");
    std::fs::write(&file, b"precious")?;
    let lookalike = dir.path().join("lookalike");
    std::fs::create_dir(&lookalike)?;
    std::fs::write(
        lookalike.join("scene.json"),
        br#"{"format":"something else"}"#,
    )?;

    for target in [&documents, &file, &lookalike] {
        let error = write(&assembly, &archive, target, &never)
            .err()
            .ok_or("must fail")?;
        assert!(
            matches!(error, PackageError::NotAPackage { .. }),
            "{target:?}: {error}"
        );
    }

    assert_eq!(std::fs::read(documents.join("thesis.txt"))?, b"precious");
    assert_eq!(std::fs::read(&file)?, b"precious");
    assert!(lookalike.join("scene.json").is_file());
    assert_eq!(
        names_in(dir.path())?,
        ["documents", "file", "lookalike"],
        "no temporary folder remains"
    );
    Ok(())
}

#[test]
fn a_folder_inside_a_protected_installation_is_refused_before_anything_is_written() -> TestResult {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let dir = tempfile::tempdir()?;
    let installation = dir.path().join("StarCitizen");
    std::fs::create_dir(&installation)?;
    p4kx_guard::protect(&installation);
    let out = installation.join("LIVE").join("package");

    let error = write(&assembly, &archive, &out, &never)
        .err()
        .ok_or("must fail")?;

    assert!(matches!(error, PackageError::Refused(_)), "{error}");
    assert!(names_in(&installation)?.is_empty(), "nothing was created");
    Ok(())
}

#[test]
fn a_cancelled_run_leaves_no_output_and_keeps_an_earlier_package() -> TestResult {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let dir = tempfile::tempdir()?;
    let fresh = dir.path().join("fresh");
    let earlier = dir.path().join("earlier");
    write(&assembly, &archive, &earlier, &never)?;
    let before = std::fs::read(earlier.join("scene.json"))?;

    for polls in [0, 1, 2, 4, 7] {
        let seen = AtomicUsize::new(0);
        let cancel_after = |count: usize| seen.fetch_add(1, Ordering::Relaxed) >= count;
        for target in [&fresh, &earlier] {
            seen.store(0, Ordering::Relaxed);
            let error = write(&assembly, &archive, target, &|| cancel_after(polls))
                .err()
                .ok_or_else(|| format!("polls {polls}: must be cancelled"))?;
            assert!(matches!(error, PackageError::Cancelled), "{error}");
        }
    }

    assert_eq!(
        names_in(dir.path())?,
        ["earlier"],
        "no output, no temporary folder"
    );
    assert_eq!(std::fs::read(earlier.join("scene.json"))?, before);
    Ok(())
}

#[test]
fn a_texture_that_cannot_be_loaded_is_an_issue_not_a_failure() -> TestResult {
    let fixture = fixture(livery_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, "TEST_Livery")?;
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("ship");

    let summary = write_package(
        &assembly,
        &Failing,
        &glb_options(0),
        &PackageInfo::default(),
        &out,
        &never,
    )?;

    assert_eq!(summary.textures, 0);
    assert_eq!(summary.texture_failures, 1);
    assert_eq!(names_in(&out.join("textures"))?, Vec::<String>::new());
    let text = std::fs::read_to_string(out.join("scene.json"))?;
    let manifest: p4kx_blender::Manifest = serde_json::from_str(&text)?;
    let failure = manifest
        .issues
        .iter()
        .find(|issue| issue.origin == "texture")
        .ok_or("a texture issue")?;
    assert_eq!(failure.kind, "texture");
    assert!(
        failure.reason.contains("broken on purpose"),
        "{}",
        failure.reason
    );
    assert!(
        manifest
            .parts
            .iter()
            .flat_map(|part| &part.materials)
            .all(|material| material.textures.diffuse.is_none()),
        "no material names a texture that is not there"
    );
    Ok(())
}
