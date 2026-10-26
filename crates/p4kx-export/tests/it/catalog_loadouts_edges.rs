// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Loadouts of the Basetool catalog through the public API: an Entity that holds itself and an
//! empty port with a nested Loadout.

use crate::loadouts_fixture::{GUN, LOOP, NESTED};
use crate::loadouts_support::{build, guid, loadout, names, port};

use std::error::Error;

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn an_entity_that_holds_itself_is_an_unresolved_cycle_and_is_not_attached() -> TestResult {
    let (built, catalog) = build(None)?;
    let entry = loadout(&catalog, LOOP)?;
    let held = port(&entry["ports"], "loop_1")?;
    assert_eq!(
        (
            &held["unresolved"],
            &held["itemGuid"],
            &held["itemClassName"],
            &held["ports"]
        ),
        (
            &json!("cycle"),
            &json!(guid(LOOP)),
            &Value::Null,
            &json!([])
        ),
        "the cycle is cut at the port and the GUID the Loadout names is kept"
    );
    assert_eq!(entry["issues"], json!(["cycle"]), "the code is listed");
    let summary = built.summary().loadouts();
    assert_eq!(
        (summary.unresolved, summary.with_issues),
        (3, 3),
        "counted as unresolved"
    );
    Ok(())
}

#[test]
fn an_empty_port_lists_the_ports_of_its_nested_loadout() -> TestResult {
    let (_, catalog) = build(None)?;
    let body = port(&loadout(&catalog, NESTED)?["ports"], "body")?;
    assert_eq!(
        (
            &body["itemGuid"],
            &body["itemClassName"],
            &body["unresolved"]
        ),
        (&Value::Null, &Value::Null, &Value::Null),
        "the empty port holds nothing itself"
    );
    assert_eq!(names(&body["ports"]), ["seat_1"], "the nested port");
    let seat = port(&body["ports"], "seat_1")?;
    assert_eq!(
        (&seat["itemGuid"], &seat["itemClassName"]),
        (&json!(guid(GUN)), &json!("ACME_Gun")),
        "and what the nested Loadout attaches to it"
    );
    Ok(())
}
