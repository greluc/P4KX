// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_assembly::ChangedLoadout;
use serde::Serialize;

use crate::query_table::one_line;

/// A replacement that was applied although its Entity does not fit the port, as the JSON lists it.
#[derive(Serialize)]
pub(crate) struct IncompatibleJson<'a> {
    port: &'a str,
    entity: &'a str,
    kind: &'static str,
    reason: String,
}

/// Returns the replacements of `applied` that do not fit their port, for the JSON.
pub(crate) fn incompatible(applied: &ChangedLoadout) -> Vec<IncompatibleJson<'_>> {
    applied
        .mismatches()
        .iter()
        .map(|mismatch| IncompatibleJson {
            port: mismatch.path(),
            entity: mismatch.entity(),
            kind: mismatch.reason().kind(),
            reason: mismatch.reason().to_string(),
        })
        .collect()
}

/// Warns on stderr about the Loadout changes that lie below a docked vehicle that is not
/// expanded and so were not applied, and about the replacements that do not fit their port and
/// were applied all the same.
pub(crate) fn warn(applied: &ChangedLoadout) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for port in applied.ignored() {
        writeln!(
            err,
            "warning: {} was not applied: the port lies below a docked vehicle that is not expanded",
            one_line(port)
        )?;
    }
    for mismatch in applied.mismatches() {
        writeln!(
            err,
            "warning: {} does not fit the port {}: {}; it was attached all the same",
            one_line(mismatch.entity()),
            one_line(mismatch.path()),
            mismatch.reason()
        )?;
    }
    Ok(())
}
