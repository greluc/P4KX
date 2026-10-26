// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A selection nested deeper than the reader follows fails its blueprint.

use std::error::Error;

use crate::crafting_detail_fixture as fixture;
use p4kx_export::read_crafting;
use p4kx_extraction::{DataForge, Guid};

#[test]
fn a_selection_nested_too_deep_fails_its_blueprint_instead_of_vanishing()
-> Result<(), Box<dyn Error>> {
    let bytes = fixture::too_deep_database().build();
    let forge = DataForge::open(&bytes)?;
    let crafting = read_crafting(&forge, None, None)?;
    let value = fixture::TOO_DEEP;
    let deep: Guid = format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        value >> 96,
        (value >> 80) & 0xffff,
        (value >> 64) & 0xffff,
        (value >> 48) & 0xffff,
        value & 0xffff_ffff_ffff
    )
    .parse()?;
    assert!(
        crafting
            .blueprints
            .iter()
            .all(|blueprint| blueprint.guid != deep),
        "the blueprint is left out, not kept with a cut cost tree"
    );
    let [failure] = crafting.failures.as_slice() else {
        return Err(format!("one failure, found {:?}", crafting.failures).into());
    };
    assert_eq!(failure.guid, deep);
    assert!(failure.reason.contains("deeper"), "{}", failure.reason);
    Ok(())
}
