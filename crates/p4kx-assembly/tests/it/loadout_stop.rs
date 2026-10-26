// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Stopping the resolution of a Loadout from outside.

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{ChangeError, EntityNode, LoadoutChanges, LoadoutResolver};
use p4kx_dataforge::DataForge;

use crate::loadout_changes::{database, entities};

type TestResult = Result<(), Box<dyn Error>>;

fn resolve(
    changes: &LoadoutChanges,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<EntityNode, Box<dyn Error>> {
    let bytes = database(&entities()).build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge).with_stop(stop);
    let ship = resolver.entities().find("TEST_Ship")?;
    Ok(resolver.resolve_with(&ship, changes)?)
}

fn is_cancelled(outcome: Result<EntityNode, Box<dyn Error>>) -> bool {
    outcome.is_err_and(|error| {
        matches!(
            error.downcast_ref::<ChangeError>(),
            Some(ChangeError::Cancelled)
        )
    })
}

#[test]
fn a_resolution_that_is_not_stopped_completes() -> TestResult {
    let tree = resolve(&LoadoutChanges::new(), &|| false)?;
    assert_eq!(tree.class_name(), "TEST_Ship");
    assert!(tree.walk().len() > 3, "the ports are resolved");
    Ok(())
}

#[test]
fn a_stopped_resolution_is_cancelled_without_changes() {
    assert!(is_cancelled(resolve(&LoadoutChanges::new(), &|| true)));
}

#[test]
fn a_stopped_resolution_is_cancelled_with_changes() {
    let changes = LoadoutChanges::new().exclude("weapon_b");
    assert!(is_cancelled(resolve(&changes, &|| true)));
}

#[test]
fn a_resolution_stopped_while_it_runs_is_cancelled() {
    let polls = AtomicUsize::new(0);
    let stop = || polls.fetch_add(1, Ordering::Relaxed) >= 2;
    assert!(is_cancelled(resolve(&LoadoutChanges::new(), &stop)));
    assert!(polls.load(Ordering::Relaxed) >= 3, "polled while building");
}
