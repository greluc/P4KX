//! Whether the GUI looks for a newer Release at start.

use std::error::Error;

use p4kx_settings::Settings;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_update_check_is_off_until_switched_on() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "[channels]\nfolders = []\n")?;

    let settings = Settings::load(&file)?;

    assert!(!settings.update_check, "off without an entry");
    assert!(!Settings::default().update_check, "off by default");
    Ok(())
}

#[test]
fn a_switched_on_update_check_survives_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        update_check: true,
        ..Settings::default()
    };

    settings.save(&file)?;

    assert!(
        std::fs::read_to_string(&file)?.contains("update_check = true"),
        "written as a plain key"
    );
    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}
