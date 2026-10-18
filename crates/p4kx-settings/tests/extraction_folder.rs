//! The folder, the Conversion switch and the texture outputs the GUI last chose for an Extraction, kept in the
//! settings file.

use std::error::Error;
use std::path::PathBuf;

use p4kx_settings::{ExtractionTextures, Settings, WindowGeometry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_extraction_folder_survives_a_round_trip_next_to_the_tables() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut settings = Settings {
        extraction_folder: Some(PathBuf::from(r"D:\Exports")),
        extraction_conversion: true,
        window: Some(WindowGeometry {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            maximized: false,
        }),
        ..Settings::default()
    };
    settings.channels.add_folder(PathBuf::from(r"D:\Games"));

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_file_without_an_extraction_folder_leaves_it_unset() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;

    assert_eq!(Settings::load(&file)?.extraction_folder, None, "unset");
    Ok(())
}

#[test]
fn a_file_without_the_conversion_switch_leaves_it_off() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;

    assert!(!Settings::load(&file)?.extraction_conversion, "off");
    Ok(())
}

#[test]
fn the_texture_outputs_survive_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        extraction_conversion: true,
        extraction_textures: ExtractionTextures {
            merge: false,
            png: true,
            mip: 2,
            keep_raw: true,
        },
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_file_without_texture_outputs_merges_and_exports_the_largest_mip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "[extraction_textures]\nkeep_raw = true\n")?;

    assert_eq!(
        Settings::load(&file)?.extraction_textures,
        ExtractionTextures {
            merge: true,
            png: true,
            mip: 0,
            keep_raw: true,
        },
        "the missing fields take their defaults"
    );
    assert_eq!(
        Settings::default().extraction_textures,
        ExtractionTextures {
            merge: true,
            png: true,
            mip: 0,
            keep_raw: false,
        },
        "the defaults"
    );
    Ok(())
}
