// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Entities tab in the window, driven headless over a synthetic Archive with a ship, a
//! vehicle, weapons and a shield.

use crate::entity_picking;
use crate::tagging;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use entity_picking::{TestResult, opened, ports, select, settle, shown};
use p4kx_gui::{Language, Model, RowChange, WorkTab};
use p4kx_testkit::TestEntry;

fn type_search(harness: &mut egui_kittest::Harness<'_, Model>, text: &str) {
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);
    harness.get_by_role(Role::TextInput).type_text(text);
    harness.run_steps(3);
}

#[test]
fn the_tab_sits_next_to_the_others_and_asks_for_an_archive() {
    let mut harness = tagging::harness(Model::new(Language::BritishEnglish));
    harness.run_steps(2);

    harness.get_by_role_and_label(Role::Tab, "Entities").click();
    harness.run_steps(2);

    assert_eq!(harness.state().work_tab(), WorkTab::Entities, "switched");
    assert!(
        harness.query_by_label("No Archive open").is_some(),
        "asks for an Archive"
    );
}

#[test]
fn the_picker_lists_ships_vehicles_and_weapons_but_no_other_item() -> TestResult {
    let (_dir, harness) = opened(Language::BritishEnglish)?;

    assert_eq!(
        shown(harness.state()),
        [
            "TEST_Cart",
            "TEST_Gun_S1",
            "TEST_Gun_S2",
            "TEST_Gun_S3",
            "TEST_Gun_S4",
            "TEST_Ship"
        ],
        "the relay and the shield are no pick"
    );
    assert!(harness.query_by_label("6 Entities").is_some(), "the count");
    Ok(())
}

#[test]
fn the_search_matches_the_name_and_the_record_path_and_the_kinds_narrow_the_list() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;

    type_search(&mut harness, "gun_s3");
    assert_eq!(
        shown(harness.state()),
        ["TEST_Gun_S3"],
        "by name, ignoring case"
    );
    harness.state_mut().entities_mut().set_query("zzz");
    harness.run_steps(3);
    assert!(
        harness
            .query_by_label("No Entity matches the search.")
            .is_some(),
        "nothing matches"
    );

    harness
        .state_mut()
        .entities_mut()
        .set_query("groundvehicles");
    harness.run_steps(2);
    assert_eq!(shown(harness.state()), ["TEST_Cart"], "by record path");

    harness.state_mut().entities_mut().set_query("");
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Weapons 4")
        .click();
    harness.run_steps(3);
    assert_eq!(
        shown(harness.state()),
        ["TEST_Gun_S1", "TEST_Gun_S2", "TEST_Gun_S3", "TEST_Gun_S4"],
        "weapons only"
    );
    Ok(())
}

#[test]
fn selecting_an_entity_shows_its_loadout_tree_with_ports_and_attached_entities() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "TEST_Ship")?;
    harness.run_steps(3);

    assert_eq!(
        ports(harness.state()),
        [
            ("hardpoint_weapon_nose".to_owned(), RowChange::Kept),
            ("hardpoint_relay".to_owned(), RowChange::Kept)
        ],
        "both ports"
    );
    assert!(
        harness.query_by_label_contains("TEST_Gun_S2").is_some(),
        "the attached Entity is named"
    );
    assert!(harness.query_by_label("The default Loadout").is_some());
    Ok(())
}

#[test]
fn unchecking_a_port_leaves_its_branch_out_and_checking_it_again_takes_that_back() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;

    harness
        .get_by_role_and_label(Role::CheckBox, "Include hardpoint_weapon_nose")
        .click();
    settle(&mut harness)?;

    assert_eq!(
        ports(harness.state())
            .iter()
            .find(|(path, _)| path == "hardpoint_weapon_nose")
            .map(|(_, change)| *change),
        Some(RowChange::Excluded)
    );
    assert!(harness.query_by_label("1 left out, 0 replaced").is_some());
    harness
        .get_by_role_and_label(Role::CheckBox, "Include hardpoint_weapon_nose")
        .click();
    settle(&mut harness)?;
    assert!(harness.query_by_label("The default Loadout").is_some());
    Ok(())
}

#[test]
fn a_replacement_offers_only_the_entities_that_fit_the_port() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;

    harness
        .get_all_by_role_and_label(Role::Button, "Replace…")
        .next()
        .ok_or("no replace action")?
        .click();
    harness.run_steps(3);

    let chooser = harness.state().entities().replace().ok_or("no chooser")?;
    let fitting: Vec<&str> = (0..chooser.shown_len())
        .filter_map(|at| chooser.get(at))
        .map(p4kx_assembly::EntitySummary::class_name)
        .collect();
    assert_eq!(
        fitting,
        ["TEST_Gun_S1", "TEST_Gun_S2", "TEST_Gun_S3"],
        "no size 4 gun, no shield"
    );
    assert!(
        harness
            .query_by_label("The port takes WeaponGun (Gun), sizes 1\u{2013}3.")
            .is_some()
    );
    harness
        .get_all_by_role_and_label(Role::Button, "TEST_Gun_S3")
        .last()
        .ok_or("the gun is not offered")?
        .click();
    settle(&mut harness)?;

    assert!(harness.state().entities().replace().is_none(), "closed");
    assert_eq!(
        ports(harness.state())
            .iter()
            .find(|(path, _)| path == "hardpoint_weapon_nose")
            .map(|(_, change)| *change),
        Some(RowChange::Replaced)
    );
    assert!(harness.query_by_label_contains("TEST_Gun_S3").is_some());
    Ok(())
}

#[test]
fn a_port_with_sizes_only_offers_the_entities_without_a_size_or_of_that_size() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;

    harness
        .state_mut()
        .entities_mut()
        .begin_replace("hardpoint_relay");
    harness.run_steps(3);

    assert!(
        harness.query_by_label("The port takes size 1.").is_some(),
        "the sizes are named"
    );
    let chooser = harness.state().entities().replace().ok_or("no chooser")?;
    assert_eq!(
        chooser.fitting(),
        3,
        "the ship, the cart and the size 1 gun"
    );
    Ok(())
}

#[test]
fn an_archive_without_a_dataforge_fails_with_the_reason() -> TestResult {
    let (_dir, harness) = entity_picking::opened_sized(
        vec![TestEntry::stored("readme.txt", "readme")],
        Language::BritishEnglish,
        [1400.0, 900.0],
    )?;

    assert!(
        harness
            .query_by_label_contains("The Entities cannot be listed")
            .is_some(),
        "the reason is shown"
    );
    Ok(())
}

#[test]
fn the_tab_is_german_in_the_german_window() -> TestResult {
    let (_dir, mut harness) = opened(Language::German)?;
    select(&mut harness, "TEST_Ship")?;

    assert!(harness.query_by_label("6 Entities").is_some());
    assert!(harness.query_by_label("Das Standard-Loadout").is_some());
    assert!(
        harness
            .query_by_role_and_label(Role::CheckBox, "hardpoint_relay einbeziehen")
            .is_some()
    );
    Ok(())
}
