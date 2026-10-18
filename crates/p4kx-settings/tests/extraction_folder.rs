//! The folder the GUI last chose for an Extraction, kept in the settings file.

use std::error::Error;
use std::path::PathBuf;

use p4kx_settings::{Settings, WindowGeometry};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_extraction_folder_survives_a_round_trip_next_to_the_tables() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut settings = Settings {
        extraction_folder: Some(PathBuf::from(r"D:\Exports")),
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
