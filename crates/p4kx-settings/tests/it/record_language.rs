// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The language Record previews show localisation keys in.

use std::error::Error;

use p4kx_settings::Settings;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_record_language_follows_the_interface_until_chosen() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "[channels]\nfolders = []\n")?;

    let settings = Settings::load(&file)?;

    assert_eq!(settings.record_language, None, "unset without an entry");
    settings.save(&file)?;
    assert!(
        !std::fs::read_to_string(&file)?.contains("record_language"),
        "an unset choice is not written"
    );
    Ok(())
}

#[test]
fn a_chosen_record_language_survives_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        record_language: Some("de".to_owned()),
        ..Settings::default()
    };

    settings.save(&file)?;

    assert!(
        std::fs::read_to_string(&file)?.contains("record_language = \"de\""),
        "written as a plain key"
    );
    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}
