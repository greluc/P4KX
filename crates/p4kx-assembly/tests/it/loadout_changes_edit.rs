// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Editing [`LoadoutChanges`] in place and the port constraints a port node keeps.

use std::error::Error;

use p4kx_assembly::{ChangeError, ItemFacts, LoadoutChanges, PortChange};

use crate::loadout_changes::{applied, port};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_port_without_changes_is_kept() {
    let changes = LoadoutChanges::new();
    assert_eq!(changes.action_for("weapon_a"), PortChange::Keep);
    assert_eq!(changes.action_for(""), PortChange::Keep, "no path");
}

#[test]
fn an_exclusion_is_set_found_ignoring_case_and_removed() -> TestResult {
    let mut changes = LoadoutChanges::new();
    changes.set_excluded("Weapon_B/Mount_Gun")?;

    assert_eq!(
        changes.action_for("weapon_b/mount_gun"),
        PortChange::Exclude
    );
    assert_eq!(changes.action_for("weapon_b"), PortChange::Keep);
    assert!(changes.remove(" WEAPON_B/mount_gun "));
    assert!(changes.is_empty());
    assert!(
        !changes.remove("weapon_b/mount_gun"),
        "nothing left to remove"
    );
    Ok(())
}

#[test]
fn a_replacement_is_set_found_and_replaced_in_place() -> TestResult {
    let mut changes = LoadoutChanges::new();
    changes.set_replacement("weapon_a", "GUN_S1")?;
    changes.set_replacement("WEAPON_A", "GUN_S2")?;

    assert_eq!(
        changes.action_for("weapon_a"),
        PortChange::Replace("GUN_S2")
    );
    assert_eq!(changes.replacements().len(), 1, "one change per port");
    Ok(())
}

#[test]
fn an_exclusion_and_a_replacement_of_one_port_displace_each_other() -> TestResult {
    let mut changes = LoadoutChanges::new();
    changes.set_replacement("weapon_a", "GUN_S1")?;
    changes.set_excluded("weapon_a")?;
    assert_eq!(changes.action_for("weapon_a"), PortChange::Exclude);
    assert_eq!(changes.replacements(), &[] as &[(String, String)]);

    changes.set_replacement("weapon_a", "GUN_S2")?;
    assert_eq!(
        changes.action_for("weapon_a"),
        PortChange::Replace("GUN_S2")
    );
    assert_eq!(changes.exclusions(), &[] as &[String]);
    Ok(())
}

#[test]
fn a_change_drops_the_changes_below_the_port() -> TestResult {
    let mut changes = LoadoutChanges::new();
    changes.set_excluded("weapon_b/mount_gun")?;
    changes.set_replacement("weapon_b/other", "GUN_S1")?;
    changes.set_excluded("power")?;

    changes.set_excluded("weapon_b")?;

    assert_eq!(changes.action_for("weapon_b"), PortChange::Exclude);
    assert_eq!(changes.action_for("weapon_b/mount_gun"), PortChange::Keep);
    assert_eq!(changes.action_for("weapon_b/other"), PortChange::Keep);
    assert_eq!(
        changes.action_for("power"),
        PortChange::Exclude,
        "the rest stays"
    );
    Ok(())
}

#[test]
fn taking_back_a_replacement_drops_the_changes_below_it() -> TestResult {
    let mut changes = LoadoutChanges::new();
    changes.set_replacement("weapon_b", "MOUNT")?;
    changes.set_excluded("weapon_b/mount_gun")?;
    changes.set_excluded("power")?;

    assert!(changes.remove("weapon_b"));

    assert_eq!(changes.action_for("weapon_b/mount_gun"), PortChange::Keep);
    assert_eq!(
        changes.action_for("power"),
        PortChange::Exclude,
        "the rest stays"
    );
    assert!(
        applied(&changes).is_ok(),
        "no change is left that names a port of the replaced Entity"
    );
    Ok(())
}

#[test]
fn taking_back_an_exclusion_keeps_the_other_changes() -> TestResult {
    let mut changes = LoadoutChanges::new();
    changes.set_excluded("weapon_a")?;
    changes.set_excluded("weapon_b/mount_gun")?;

    assert!(changes.remove("weapon_a"));

    assert_eq!(
        changes.action_for("weapon_b/mount_gun"),
        PortChange::Exclude
    );
    Ok(())
}

#[test]
fn edited_changes_resolve_like_built_ones() -> TestResult {
    let mut edited = LoadoutChanges::new();
    edited.set_excluded("weapon_b")?;
    edited.set_replacement("weapon_a", "GUN_S2")?;

    let tree = applied(&edited)?;
    let built = applied(
        &LoadoutChanges::new()
            .exclude("weapon_b")
            .replace("weapon_a", "GUN_S2"),
    )?;

    assert_eq!(tree, built);
    Ok(())
}

#[test]
fn a_path_that_is_none_cannot_be_set() {
    let mut changes = LoadoutChanges::new();
    assert!(matches!(
        changes.set_excluded("a//b"),
        Err(ChangeError::InvalidPort { .. })
    ));
    assert!(matches!(
        changes.set_replacement(" ", "GUN_S1"),
        Err(ChangeError::InvalidPort { .. })
    ));
    assert!(changes.is_empty());
}

#[test]
fn a_port_keeps_its_constraints_for_many_candidates() -> TestResult {
    let tree = applied(&LoadoutChanges::new())?;
    let weapon = port(&tree, "weapon_a")?;

    let first = weapon.constraints().ok_or("no constraints")?;
    let second = weapon.constraints().ok_or("no constraints")?;
    assert!(std::ptr::eq(first, second), "built once, not per call");
    assert_eq!((first.min_size(), first.max_size()), (Some(1), Some(2)));

    let tree_entities: Vec<ItemFacts> = ["GUN_S1", "GUN_S2", "GUN_S3", "SHIELD_S1"]
        .iter()
        .map(|name| {
            let tree = applied(&LoadoutChanges::new().replace("cargo", *name))?;
            let held = port(&tree, "cargo")?;
            match held.attachment() {
                p4kx_assembly::Attachment::Entity(node) => Ok(node.item_facts()),
                _ => Err("no entity".into()),
            }
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    let fitting: Vec<&str> = tree_entities
        .iter()
        .filter(|facts| weapon.accepts(facts).is_ok())
        .map(ItemFacts::class_name)
        .collect();
    assert_eq!(fitting, ["GUN_S1", "GUN_S2"]);
    Ok(())
}
