// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The description of contracts in the Basetool catalog.

use crate::catalog_contracts::{A, C, E, TestResult, build, entry};

use serde_json::json;

#[test]
fn the_description_is_resolved_like_the_title_and_falls_back_to_the_broker_entry() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        entry(&catalog, C, None)?["description"],
        json!({
            "key": "@desc_handler",
            "en": "Wanted [MISSING].\nReward on delivery.",
            "de": null
        }),
        "tokens are replaced, line breaks decoded and the blanks removed"
    );
    assert_eq!(entry(&catalog, E, None)?["description"]["en"], "Haul it");
    assert_eq!(
        entry(&catalog, A, None)?["description"],
        json!({"key": null, "en": null, "de": null}),
        "a contract that states none"
    );
    Ok(())
}
