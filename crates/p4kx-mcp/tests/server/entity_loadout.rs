// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_testkit::{SAMPLE_SHIP, TestEntry, sample_ship};
use serde_json::{Value, json};

use crate::support::{Install, TestResult};

fn install() -> TestResult<Install> {
    Install::new(vec![TestEntry::zstd(
        "Data\\Game2.dcb",
        sample_ship().build(),
    )])
}

fn port<'a>(loadout: &'a Value, path: &str) -> TestResult<&'a Value> {
    loadout["ports"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|port| port["path"] == path)
        .ok_or_else(|| format!("no port {path}: {loadout}").into())
}

#[test]
fn the_loadout_tree_lists_every_port_with_what_it_holds() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let loadout = server.ok("entity_loadout", &json!({ "entity": "test_ship" }))?;

    assert_eq!(
        loadout["entity"]["class_name"], "TEST_Ship",
        "the root Entity"
    );
    assert_eq!(
        loadout["entity"]["guid"], "0079c5d5-1678-4f8c-85ba-18ca8f642af6",
        "the GUID of the sample ship {SAMPLE_SHIP:x}"
    );
    let seat = port(&loadout, "hardpoint_seat_pilot")?;
    assert_eq!(
        seat["entity"]["class_name"], "TEST_Seat",
        "attached by class name"
    );
    assert_eq!(seat["source"], "loadout", "from the Entity's Loadout");
    let nose = port(&loadout, "hardpoint_weapon_nose")?;
    assert_eq!(
        nose["definition"]["min_size"], 2,
        "the sizes of the definition"
    );
    assert_eq!(
        nose["definition"]["types"][0]["name"], "WeaponGun",
        "the types it takes"
    );
    port(&loadout, "hardpoint_weapon_nose/hardpoint_class_2")?;
    assert_eq!(
        port(&loadout, "hardpoint_paint")?.get("entity"),
        None,
        "an empty port holds nothing"
    );
    let counts = &loadout["counts"];
    assert_eq!(counts["ports"], loadout["total"], "all ports are listed");
    assert!(counts["unresolved"].as_u64().unwrap_or(0) >= 2, "{counts}");
    Ok(())
}

#[test]
fn every_unresolved_port_names_its_reason_and_target() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let loadout = server.ok("entity_loadout", &json!({ "entity": "TEST_Ship" }))?;

    let dangling = port(&loadout, "hardpoint_cargo")?;
    assert_eq!(
        dangling["unresolved"]["kind"], "dangling_reference",
        "the kind"
    );
    assert_eq!(
        dangling["unresolved"]["reference"], "00000001-0002-0003-0004-000000000005",
        "the GUID the Loadout names"
    );
    let unknown = port(&loadout, "hardpoint_door")?;
    assert_eq!(unknown["unresolved"]["kind"], "unknown_class", "the kind");
    assert_eq!(
        unknown["unresolved"]["class_name"], "TEST_No_Such_Door",
        "the class"
    );
    assert!(
        unknown["unresolved"]["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "a reason in words"
    );
    Ok(())
}

#[test]
fn max_depth_leaves_deeper_ports_out_and_says_how_many() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let full = server.ok("entity_loadout", &json!({ "entity": "TEST_Ship" }))?;

    let shallow = server.ok(
        "entity_loadout",
        &json!({ "entity": "TEST_Ship", "max_depth": 1 }),
    )?;

    let deeper = shallow["deeper"].as_u64().ok_or("deeper")?;
    assert!(deeper > 0, "the nested ports are left out");
    assert_eq!(
        shallow["total"].as_u64().ok_or("total")? + deeper,
        full["total"].as_u64().ok_or("total")?,
        "nothing is lost"
    );
    assert_eq!(
        shallow["counts"], full["counts"],
        "the counts cover the whole tree"
    );
    for entry in shallow["ports"].as_array().into_iter().flatten() {
        assert_eq!(entry["depth"], 1, "only top level ports");
    }
    Ok(())
}

#[test]
fn ports_page_with_cursors_in_tree_order() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let full = server.ok("entity_loadout", &json!({ "entity": "TEST_Ship" }))?;
    let mut paths = Vec::new();
    let mut arguments = json!({ "entity": "TEST_Ship", "limit": 2 });

    loop {
        let page = server.ok("entity_loadout", &arguments)?;
        for port in page["ports"].as_array().into_iter().flatten() {
            paths.push(port["path"].clone());
        }
        match page["next_cursor"].as_str() {
            Some(cursor) => arguments["cursor"] = json!(cursor),
            None => break,
        }
    }

    let all: Vec<Value> = full["ports"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|port| port["path"].clone())
        .collect();
    assert_eq!(paths, all, "the pages add up to the whole list");
    Ok(())
}

#[test]
fn an_unknown_entity_is_an_error() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let error = server.error("entity_loadout", &json!({ "entity": "TEST_Nothing" }))?;

    assert!(error.contains("no Entity has the record path"), "{error}");
    Ok(())
}
