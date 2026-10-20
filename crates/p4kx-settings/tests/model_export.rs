//! The choices of the Export as GLB dialog in the settings file.

use std::error::Error;
use std::path::PathBuf;

use p4kx_settings::{ModelExportSettings, ModelMaterials, Settings};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_export_choices_survive_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        model_export: ModelExportSettings {
            lod: 2,
            materials: ModelMaterials::Colours,
            vertex_colours: false,
            folder: Some(PathBuf::from("D:\\Models")),
        },
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_file_without_export_choices_gets_the_defaults() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;

    let loaded = Settings::load(&file)?.model_export;

    assert_eq!(loaded.lod, 0, "most detailed level");
    assert!(loaded.vertex_colours, "vertex colours on");
    assert_eq!(loaded.materials, ModelMaterials::Colours, "colours");
    assert_eq!(loaded.folder, None, "no folder");
    Ok(())
}
