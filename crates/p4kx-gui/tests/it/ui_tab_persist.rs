// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The game state and the typed game variables of the UI tab are kept across sessions.

use p4kx_gui::{Language, Model, UiPower};
use p4kx_settings::{MAX_VARIABLES_BYTES, Settings, UiRenderPower};

#[test]
fn the_state_and_the_typed_variables_survive_a_session() -> Result<(), Box<dyn std::error::Error>> {
    let mut first = Model::new(Language::BritishEnglish);
    first.ui_mut().set_power(UiPower::PoweredOn);
    first
        .ui_mut()
        .set_variables_text("canvas::isradar=1 powerstate=0");
    let stored = Settings {
        ui: first.ui_render_settings(),
        ..Settings::default()
    };
    assert_eq!(stored.ui.power, UiRenderPower::PoweredOn);

    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    stored.save(&file)?;
    let mut second = Model::new(Language::BritishEnglish);
    second.set_ui_render_settings(&Settings::load(&file)?.ui);

    assert_eq!(second.ui().settings().power, UiPower::PoweredOn);
    assert_eq!(
        second.ui().variables_text(),
        "canvas::isradar=1 powerstate=0"
    );
    Ok(())
}

#[test]
fn the_typed_variables_are_held_to_the_byte_limit_without_cutting_a_setting() {
    let mut model = Model::new(Language::BritishEnglish);
    model.ui_mut().set_variables_text(&"äa=1 ".repeat(5000));
    let text = model.ui().variables_text();
    assert!(text.len() <= MAX_VARIABLES_BYTES);
    assert!(text.ends_with("äa=1"));
}
