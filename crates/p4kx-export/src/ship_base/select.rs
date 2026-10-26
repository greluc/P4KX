// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{DataForgeError, Record};
use rayon::prelude::*;

use super::ShipBaseOptions;
use crate::selection::is_leftover;
use crate::{CatalogFailure, Kind};

/// The ship Records to read, the development leftovers counted and the Records that cannot be
/// checked.
pub(super) struct Selection<'f> {
    pub(super) ships: Vec<Record<'f>>,
    pub(super) skipped: usize,
    pub(super) failures: Vec<CatalogFailure>,
}

enum Check {
    Ship,
    Skipped,
    Other,
}

/// Picks the Records of the catalog's `ships` kind: the Entity Records under
/// `entities/spaceships` and `entities/groundvehicles` that the Filter matches.
pub(super) fn ships<'f>(records: &[Record<'f>], options: &ShipBaseOptions<'_>) -> Selection<'f> {
    let checked: Vec<(Record<'f>, Result<Check, CatalogFailure>)> = records
        .par_iter()
        .map(|record| (*record, check(record, options)))
        .collect();
    let mut selection = Selection {
        ships: Vec::new(),
        skipped: 0,
        failures: Vec::new(),
    };
    for (record, result) in checked {
        match result {
            Ok(Check::Ship) => selection.ships.push(record),
            Ok(Check::Skipped) => selection.skipped = selection.skipped.saturating_add(1),
            Ok(Check::Other) => {}
            Err(failure) => selection.failures.push(failure),
        }
    }
    selection
}

fn check(record: &Record<'_>, options: &ShipBaseOptions<'_>) -> Result<Check, CatalogFailure> {
    let inspect = || -> Result<Check, DataForgeError> {
        let path = record.record_path()?;
        if Kind::by_path(path) != Some(Kind::Ships) {
            return Ok(Check::Other);
        }
        if record.type_name()? != Kind::Ships.records().type_name {
            return Ok(Check::Other);
        }
        if options.filter.is_some_and(|filter| !filter.matches(path)) {
            return Ok(Check::Other);
        }
        Ok(if is_leftover(path, record.name()?) {
            Check::Skipped
        } else {
            Check::Ship
        })
    };
    inspect().map_err(|error| CatalogFailure::of(record, &error))
}
