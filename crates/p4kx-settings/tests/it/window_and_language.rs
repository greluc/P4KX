//! The language and window geometry the GUI keeps in the settings file.

use std::error::Error;

use p4kx_settings::{Settings, WindowGeometry};

type TestResult = Result<(), Box<dyn Error>>;

const GEOMETRY: WindowGeometry = WindowGeometry {
    x: -1200,
    y: 40,
    width: 1400,
    height: 900,
    maximized: true,
};

#[test]
fn language_and_window_survive_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("P4K-Extractor").join("settings.toml");
    let settings = Settings {
        language: Some("de-DE".to_owned()),
        window: Some(GEOMETRY),
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_file_without_language_and_window_leaves_them_unset() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "[channels]\nfolders = []\n")?;

    let settings = Settings::load(&file)?;

    assert_eq!(settings.language, None, "language defaults to unset");
    assert_eq!(settings.window, None, "window defaults to unset");
    Ok(())
}

#[test]
fn the_window_is_written_as_its_own_table() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        language: Some("en-GB".to_owned()),
        window: Some(GEOMETRY),
        ..Settings::default()
    };

    settings.save(&file)?;

    let text = std::fs::read_to_string(&file)?;
    assert!(text.contains("language = \"en-GB\""), "{text}");
    assert!(text.contains("[window]"), "{text}");
    Ok(())
}
