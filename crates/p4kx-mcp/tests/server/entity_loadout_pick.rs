// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `entity_loadout` with `loadout_pick` chooses the alternative of a group of a generated Loadout.

use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, assortment_record,
    entity_schema,
};
use serde_json::{Value, json};

use crate::support::{Install, TestResult};

const ASSORTMENT: u128 = 0x100;

fn body(class: &str) -> TestLoadout {
    TestLoadout::Manual(vec![TestLoadoutEntry::class("Body_ItemPort", class)])
}

fn install() -> TestResult<Install> {
    let forge = [
        TestEntity::new(1, "Beast", "entities/beast.xml")
            .loadout(TestLoadout::RandomGenerated(vec![ASSORTMENT])),
        TestEntity::new(2, "Body_Red", "entities/body_red.xml"),
        TestEntity::new(3, "Body_Blue", "entities/body_blue.xml"),
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |forge, entity| {
        forge.record(entity.record())
    })
    .record(assortment_record(
        ASSORTMENT,
        "beast",
        &[("Body", vec![body("Body_Red"), body("Body_Blue")])],
    ));
    Install::new(vec![TestEntry::zstd("Data/Game2.dcb", forge.build())])
}

fn body_class(loadout: &Value) -> Option<&str> {
    let port = loadout["ports"]
        .as_array()?
        .iter()
        .find(|port| port["path"] == "Body_ItemPort")?;
    port.get("entity")?.get("class_name")?.as_str()
}

#[test]
fn the_first_alternative_is_attached_without_a_pick() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let loadout = server.ok("entity_loadout", &json!({ "entity": "Beast" }))?;

    assert_eq!(body_class(&loadout), Some("Body_Red"), "{loadout}");
    Ok(())
}

#[test]
fn a_pick_attaches_another_alternative_and_is_part_of_the_cache_key() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let picked = server.ok(
        "entity_loadout",
        &json!({ "entity": "Beast", "loadout_pick": ["Body=1"] }),
    )?;
    let default = server.ok("entity_loadout", &json!({ "entity": "Beast" }))?;

    assert_eq!(body_class(&picked), Some("Body_Blue"), "{picked}");
    assert_eq!(body_class(&default), Some("Body_Red"), "{default}");
    Ok(())
}

#[test]
fn an_index_beyond_the_group_is_an_issue_of_the_entity() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let loadout = server.ok(
        "entity_loadout",
        &json!({ "entity": "Beast", "loadout_pick": ["Body=9"] }),
    )?;

    assert_eq!(loadout["entity"]["issues"][0]["kind"], "assortment");
    assert_eq!(body_class(&loadout), Some("Body_Red"), "{loadout}");
    Ok(())
}

#[test]
fn a_malformed_pick_is_an_error() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;

    let error = server.error(
        "entity_loadout",
        &json!({ "entity": "Beast", "loadout_pick": ["Body"] }),
    )?;

    assert!(error.contains("Body"), "{error}");
    Ok(())
}
