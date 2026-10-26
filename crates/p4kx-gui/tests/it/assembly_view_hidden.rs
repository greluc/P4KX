// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Hidden branches of the 3D view of the Entities tab.

use crate::assembly_view::{built, draw, serial, view, view_mut};
use crate::entity_picking::{TestResult, opened, select, settle};

use p4kx_gui::{AssemblyView, Language};

#[test]
fn hiding_a_branch_hides_it_without_building_again() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;
    built(&mut harness, None)?;
    let before = serial(harness.state());
    let all = draw(&mut harness)?.instances().len();

    view_mut(&mut harness)?.set_hidden("hardpoint_weapon_nose", true);
    harness.run_steps(2);

    let model = harness.state();
    let view = view(model).ok_or("no view")?;
    assert!(view.is_hidden("hardpoint_weapon_nose"), "marked");
    assert!(!view.is_building(), "nothing is built again");
    assert_eq!(serial(model), before, "the same scene");
    assert_eq!(view.hidden_parts(), 1, "one part hidden");
    let list = draw(&mut harness)?;
    assert_eq!(list.hidden_parts(), 1, "left out of the frame");
    assert!(list.instances().len() < all, "its instances are gone");

    view_mut(&mut harness)?.show_all();
    assert_eq!(draw(&mut harness)?.instances().len(), all, "shown again");
    Ok(())
}

#[test]
fn hiding_the_root_of_a_branch_hides_what_hangs_below() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;
    built(&mut harness, None)?;

    for path in ["hardpoint_weapon_nose", "hardpoint_relay"] {
        view_mut(&mut harness)?.set_hidden(path, true);
    }

    let list = draw(&mut harness)?;
    assert_eq!(list.visible_parts(), 1, "only the ship remains");
    assert_eq!(
        view(harness.state()).map(AssemblyView::hidden_count),
        Some(2)
    );
    Ok(())
}

#[test]
fn a_hidden_port_that_is_left_out_later_is_not_counted_by_show_all() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;
    built(&mut harness, None)?;
    let before = serial(harness.state());
    view_mut(&mut harness)?.set_hidden("hardpoint_weapon_nose", true);
    assert_eq!(
        view(harness.state()).map(AssemblyView::hidden_count),
        Some(1)
    );

    harness
        .state_mut()
        .entities_mut()
        .loadout_mut()
        .ok_or("no Loadout")?
        .set_excluded("hardpoint_weapon_nose", true)?;
    settle(&mut harness)?;
    built(&mut harness, before)?;

    assert_eq!(
        view(harness.state()).map(AssemblyView::hidden_count),
        Some(0),
        "the port is no longer in the view"
    );
    Ok(())
}
