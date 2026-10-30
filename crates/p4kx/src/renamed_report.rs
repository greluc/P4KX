// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::RecordRenamed;
use serde::Serialize;

/// One Record whose file does not follow its name or record path, as JSON.
#[derive(Serialize)]
pub(crate) struct RenamedJson<'a> {
    guid: String,
    path: &'a str,
    reason: String,
}

/// Returns the JSON form of `renamed`.
pub(crate) fn renamed_json(renamed: &RecordRenamed) -> RenamedJson<'_> {
    RenamedJson {
        guid: renamed.guid().to_string(),
        path: renamed.record_path(),
        reason: renamed.cause().to_string(),
    }
}

/// Writes one `renamed` line per Record of `renamed` to `out`.
pub(crate) fn write_renamed(out: &mut impl Write, renamed: &[RecordRenamed]) -> io::Result<()> {
    for note in renamed {
        writeln!(
            out,
            "renamed {} ({}): {}",
            note.record_path(),
            note.guid(),
            note.cause()
        )?;
    }
    Ok(())
}
