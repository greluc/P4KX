// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The choices of the Export of an Entity as GLB in the settings file.

use std::error::Error;
use std::path::PathBuf;

use p4kx_settings::{AssemblyExportSettings, ModelMaterials, Settings};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_export_choices_survive_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        assembly_export: AssemblyExportSettings {
            lod: 1,
            materials: ModelMaterials::Textures,
            texture_mip: 3,
            vertex_colours: false,
            interior: true,
            package: true,
            folder: Some(PathBuf::from("D:\\Ships")),
        },
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_material_mode_this_version_does_not_know_reads_as_colours() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(
        &file,
        "[assembly_export]
materials = \"pbr-from-the-future\"
lod = 2
package = true
",
    )?;

    let loaded = Settings::load(&file)?.assembly_export;

    assert_eq!(loaded.materials, ModelMaterials::Colours, "falls back");
    assert_eq!(loaded.lod, 2, "the other choices are kept");
    assert!(loaded.package, "the form is kept");
    Ok(())
}

#[test]
fn a_file_without_export_choices_gets_the_defaults() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;

    let loaded = Settings::load(&file)?.assembly_export;

    assert_eq!(loaded, AssemblyExportSettings::default(), "the defaults");
    assert_eq!(loaded.lod, 0, "most detailed level");
    assert!(loaded.vertex_colours, "vertex colours on");
    assert!(!loaded.interior, "hull only");
    assert!(!loaded.package, "a GLB file");
    assert_eq!(loaded.folder, None, "no folder");
    Ok(())
}
