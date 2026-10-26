// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_assembly::character::{Chf, NameSummary, NameTable};
use p4kx_assembly::{ArchiveModels, ForgeItems, LoadoutResolver, archive_names};
use p4kx_extraction::DataForgeEntry;

/// Returns the names of the built-in table, extended with those the Archive at `archive` knows
/// for the hashes of `chf`.
pub(crate) fn names_for(archive: Option<&Path>, chf: &Chf) -> Result<NameTable> {
    names_for_all(archive, &[chf])
}

/// Returns the names of the built-in table, extended with those the Archive at `archive` knows
/// for the hashes of any of `chfs`; the Archive is opened once.
pub(crate) fn names_for_all(archive: Option<&Path>, chfs: &[&Chf]) -> Result<NameTable> {
    let Some(path) = archive else {
        return Ok(NameTable::builtin());
    };
    let archive = crate::key::open(path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let models = ArchiveModels::new(&archive);
    let mut table = NameTable::builtin();
    for chf in chfs {
        table.merge(&archive_names(
            &chf.character,
            &mut ForgeItems::new(&mut resolver),
            &models,
        ));
    }
    Ok(table)
}

/// Returns the hashes as `0x` and eight hexadecimal digits each.
pub(crate) fn hex_list(hashes: &[u32]) -> Vec<String> {
    hashes.iter().map(|hash| format!("{hash:#010x}")).collect()
}

/// Writes how many hashes got a name to stderr, so that stdout stays the JSON.
pub(crate) fn note(summary: &NameSummary) -> io::Result<()> {
    writeln!(io::stderr().lock(), "{}", sentence(summary))
}

/// Returns the sentence that counts the named and the unresolved hashes.
pub(crate) fn sentence(summary: &NameSummary) -> String {
    let mut text = format!(
        "{} hashes named, {} unresolved",
        summary.named, summary.unresolved
    );
    if !summary.unresolved_hashes.is_empty() {
        let _infallible = write!(
            text,
            " ({} distinct: {})",
            summary.unresolved_hashes.len(),
            hex_list(&summary.unresolved_hashes).join(", ")
        );
    }
    text
}
