// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The export of a socpak container on its own, from the Entry that holds it.

use crate::entity_picking::TestResult;
use crate::interior_export::{
    TOGGLE, click_settled, container_window, export, has_node, select_socpak, summary,
};
use crate::tagging::wait_until;
use crate::waiting::act_until;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Model, TreeNode};
use p4kx_testkit::INTERIOR_ROOT;

#[test]
fn a_socpak_entry_offers_the_export_of_its_interior() -> TestResult {
    let (dir, mut harness) = container_window()?;
    assert!(!harness.state().interior_export_shown(), "nothing selected");

    select_socpak(&mut harness, INTERIOR_ROOT)?;
    assert!(harness.state().interior_export_shown());
    assert!(harness.state().interior_export_available());
    act_until(
        &mut harness,
        "the click on the Interior export to open the setup",
        |harness| click_settled(harness, "Export Interior…"),
        |model| model.assembly_export_setup().is_some(),
    )?;

    let setup = harness.state().assembly_export_setup().ok_or("no setup")?;
    assert!(setup.is_container(), "the container on its own");
    assert_eq!(setup.name(), "ship_interior");
    assert!(
        harness.query_by_label("Container: ship_interior").is_some(),
        "the setup names the container"
    );
    assert!(
        harness.query_by_label(TOGGLE).is_none(),
        "the Interior is the whole export"
    );
    harness.get_by_role_and_label(Role::Button, "LOD 1").click();
    harness.run_steps(2);

    let file = dir.path().join("ship_interior.glb");
    export(&mut harness, &file)?;

    let summary = summary(&harness)?;
    let interior = summary.interior().ok_or("no Interior")?;
    assert!(interior.containers >= 2, "the root and the cockpit");
    assert!(interior.lights >= 1, "the lamp of the root");
    assert_eq!(summary.lod(), 1);
    let bytes = std::fs::read(&file)?;
    assert_eq!(bytes.get(..4), Some(&b"glTF"[..]), "a GLB file");
    assert!(has_node(&bytes, "ship_interior")?, "the root container");
    assert!(has_node(&bytes, "cockpit_seat")?, "a model of a nested one");
    assert!(harness.query_by_label("Interior exported").is_some());
    assert!(
        harness
            .query_by_label_contains("Interior: containers opened")
            .is_some(),
        "a container on its own counts the containers opened"
    );
    let remembered = harness.state().assembly_export_settings();
    assert_eq!(remembered.lod, 1, "remembered");
    assert_eq!(remembered.folder.as_deref(), Some(dir.path()));
    Ok(())
}

fn is_cockpit(browser: &p4kx_gui::Browser, id: u32) -> bool {
    browser
        .index()
        .entry(id)
        .is_some_and(|entry| entry.path().ends_with("cockpit.socpak"))
}

/// Selects the Entry `modules\cockpit.socpak` inside the container `ship_interior.socpak`.
fn select_nested_cockpit(harness: &mut Harness<'_, Model>) -> TestResult {
    for name in ["Data", "ObjectContainers", "ships", "test"] {
        let node = tree_node(harness, name)?;
        harness
            .state_mut()
            .browser_mut()
            .ok_or("no Archive")?
            .toggle_expanded(node);
    }
    let root = tree_node(harness, "ship_interior.socpak")?;
    let id = root.as_container().ok_or("no container")?;
    harness
        .state_mut()
        .browser_mut()
        .ok_or("no Archive")?
        .toggle_expanded(root);
    wait_until(harness, |model| {
        model.browser().is_some_and(|browser| {
            !matches!(
                browser.container_status(id),
                p4kx_gui::ContainerStatus::Opening
            )
        })
    })?;
    let modules = tree_node(harness, "modules")?;
    harness
        .state_mut()
        .browser_mut()
        .ok_or("no Archive")?
        .select_folder(modules);
    wait_until(harness, |model| {
        model
            .browser()
            .is_some_and(|browser| browser.shown().iter().any(|&id| is_cockpit(browser, id)))
    })?;
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    let cockpit = browser
        .shown()
        .iter()
        .copied()
        .find(|&id| is_cockpit(browser, id))
        .ok_or("no cockpit")?;
    browser.select_entry(cockpit, || {});
    harness.run_steps(2);
    Ok(())
}

#[test]
fn a_socpak_of_another_container_is_exported_from_where_it_lies() -> TestResult {
    let (dir, mut harness) = container_window()?;
    select_nested_cockpit(&mut harness)?;
    assert!(harness.state().interior_export_available());

    harness.state_mut().begin_interior_export();
    let setup = harness.state().assembly_export_setup().ok_or("no setup")?;
    assert_eq!(setup.name(), "cockpit");
    let file = dir.path().join("cockpit.glb");
    export(&mut harness, &file)?;

    let bytes = std::fs::read(&file)?;
    assert!(
        has_node(&bytes, "cockpit")?,
        "the container the export started from"
    );
    assert!(has_node(&bytes, "cockpit_seat")?, "its model");
    Ok(())
}

fn tree_node(
    harness: &Harness<'_, Model>,
    name: &str,
) -> Result<TreeNode, Box<dyn std::error::Error>> {
    let browser = harness.state().browser().ok_or("no Archive")?;
    browser
        .tree_rows()
        .iter()
        .map(|row| row.node)
        .find(|&node| browser.node_name(node) == name)
        .ok_or_else(|| format!("no tree row {name}").into())
}

#[test]
fn an_entry_that_is_no_socpak_offers_no_interior_export() -> TestResult {
    let (_dir, mut harness) = container_window()?;
    select_socpak(&mut harness, "Data\\Objects\\Test\\cube.cgf")?;

    assert!(!harness.state().interior_export_shown());
    harness.state_mut().begin_interior_export();
    assert!(harness.state().assembly_export_setup().is_none());
    Ok(())
}
