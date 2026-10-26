// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::query_table::one_line;

/// Warns on stderr about the Loadout changes that lie below a docked vehicle that is not
/// expanded and so were not applied.
pub(crate) fn warn(ignored: &[String]) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for port in ignored {
        writeln!(
            err,
            "warning: {} was not applied: the port lies below a docked vehicle that is not expanded",
            one_line(port)
        )?;
    }
    Ok(())
}
