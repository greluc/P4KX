// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::dcb_refs_table::{count, table};
use crate::dcb_tags::{Listing, Row};
use crate::query_table::one_line;

/// Writes the tags as an aligned table: the full tag path, or with `tree` the name indented below
/// its parent, then the GUID and the legacy id.
pub(crate) fn write(out: &mut impl Write, listing: &Listing<'_>, tree: bool) -> io::Result<()> {
    let rows: Vec<Vec<String>> = listing.rows.iter().map(|row| cells(row, tree)).collect();
    let heading = if tree { "tag" } else { "tag path" };
    table(out, &[heading, "GUID", "legacy id"], &rows)?;
    writeln!(
        out,
        "{} of {}",
        count(listing.rows.len(), "tag"),
        count(listing.total, "tag")
    )
}

fn cells(row: &Row<'_>, tree: bool) -> Vec<String> {
    let label = if tree {
        format!("{}{}", "  ".repeat(row.indent), row.tag.name())
    } else {
        row.tag.path().to_owned()
    };
    vec![
        one_line(&label),
        row.tag.guid().to_string(),
        row.tag
            .legacy_id()
            .map_or_else(String::new, |id| id.to_string()),
    ]
}
