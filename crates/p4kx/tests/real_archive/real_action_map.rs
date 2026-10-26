// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The action maps of a real Archive hold a known action with its label and default binding.

use std::error::Error;

use p4kx_export::{ActionMapFormat, Device, Language, read_action_maps, write_action_maps};
use p4kx_extraction::{Archive, LocalizationSet};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_eject_action_has_its_english_label_and_default_keys() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let maps = read_action_maps(&archive)?;
    assert!(maps.maps.len() >= 40, "{} action maps", maps.maps.len());
    assert!(
        maps.action_count() >= 1000,
        "{} actions",
        maps.action_count()
    );
    let general = maps
        .maps
        .iter()
        .find(|map| map.name == "seat_general")
        .ok_or("no seat_general action map")?;
    let eject = general
        .actions
        .iter()
        .find(|action| action.name == "v_eject")
        .ok_or("no v_eject action")?;
    assert_eq!(eject.label_key.as_deref(), Some("@ui_CIEject"));
    let keyboard: Vec<&str> = eject
        .bindings
        .iter()
        .filter(|binding| binding.device == Device::Keyboard)
        .map(|binding| binding.input.as_str())
        .collect();
    assert_eq!(keyboard, ["ralt+y"]);

    let set = LocalizationSet::read(&archive, &[Language::English])?;
    let texts = set.index()?;
    let mut csv = Vec::new();
    let counts = write_action_maps(
        &maps,
        &texts,
        Language::English,
        ActionMapFormat::Csv,
        &mut csv,
    )?;
    assert_eq!(counts.actions, maps.action_count());
    let csv = String::from_utf8(csv)?;
    assert!(
        csv.lines().any(|line| line.starts_with("seat_general,")
            && line.contains(",v_eject,Eject,press,ralt+y,")),
        "no v_eject row in the CSV"
    );
    Ok(())
}
