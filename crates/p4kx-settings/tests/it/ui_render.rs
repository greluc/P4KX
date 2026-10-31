// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The choices of the UI tab in the settings file.

use std::error::Error;

use p4kx_settings::{
    MAX_VARIABLES_BYTES, Settings, UiRenderBackground, UiRenderFormat, UiRenderPower,
    UiRenderSettings, UiRenderSize, bound_variables,
};

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
            power: UiRenderPower::PoweredOn,
            variables: "isradar=1 mapdisplaymaster::ismapactive=1".to_owned(),
        },
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "settings round-trip");
    Ok(())
}

#[test]
fn typed_variables_are_cut_on_load_at_the_last_separator() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let long = "äb=1 ".repeat(MAX_VARIABLES_BYTES);
    std::fs::write(
        &file,
        format!("[ui]\npower = \"warp\"\nvariables = \"{long}\"\n"),
    )?;

    let loaded = Settings::load(&file)?.ui;

    assert_eq!(loaded.power, UiRenderPower::AtRest, "unknown state");
    assert!(loaded.variables.len() <= MAX_VARIABLES_BYTES, "bounded");
    assert!(loaded.variables.ends_with("äb=1"), "no setting is cut");
    Ok(())
}

#[test]
fn a_text_without_a_separator_that_is_too_long_becomes_empty() {
    assert_eq!(bound_variables(&"a".repeat(MAX_VARIABLES_BYTES + 1)), "");
    let exact = format!("{}=1", "a".repeat(MAX_VARIABLES_BYTES - 2));
    assert_eq!(bound_variables(&exact), exact);
    let split = format!("{} b=1", "a".repeat(MAX_VARIABLES_BYTES - 3));
    assert_eq!(bound_variables(&split).len(), MAX_VARIABLES_BYTES - 3);
    let tail = format!("{},b", "a".repeat(MAX_VARIABLES_BYTES - 1));
    assert_eq!(bound_variables(&tail).len(), MAX_VARIABLES_BYTES - 1);
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
