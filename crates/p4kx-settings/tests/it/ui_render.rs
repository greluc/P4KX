// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The choices of the UI tab in the settings file.

use std::error::Error;

use p4kx_settings::{Settings, UiRenderBackground, UiRenderFormat, UiRenderSettings, UiRenderSize};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_choices_survive_a_round_trip() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let settings = Settings {
        ui: UiRenderSettings {
            size: UiRenderSize::FullHd,
            background: UiRenderBackground::Black,
            format: UiRenderFormat::Svg,
        },
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn a_file_without_choices_gets_the_defaults() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;

    let loaded = Settings::load(&file)?.ui;

    assert_eq!(loaded, UiRenderSettings::default(), "the defaults");
    assert_eq!(loaded.size, UiRenderSize::Stage, "the stage");
    assert_eq!(
        loaded.background,
        UiRenderBackground::Transparent,
        "nothing"
    );
    assert_eq!(loaded.format, UiRenderFormat::Png, "a PNG");
    Ok(())
}

#[test]
fn a_value_this_version_does_not_know_reads_as_the_default() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(
        &file,
        "[ui]\nsize = \"wide-8192\"\nbackground = \"black\"\nformat = \"webp\"\n",
    )?;

    let loaded = Settings::load(&file)?.ui;

    assert_eq!(loaded.size, UiRenderSize::Stage, "unknown size");
    assert_eq!(loaded.background, UiRenderBackground::Black, "known value");
    assert_eq!(loaded.format, UiRenderFormat::Png, "unknown format");
    Ok(())
}
