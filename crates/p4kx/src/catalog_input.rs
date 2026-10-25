// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::Path;

use thiserror::Error;

/// An export time that no system time can hold.
#[derive(Debug, Error)]
#[error("the export time {0} lies beyond the supported range")]
pub(crate) struct InvalidExportTime(u64);

impl InvalidExportTime {
    /// Returns the error for the export time `seconds`.
    pub(crate) const fn of(seconds: u64) -> Self {
        Self(seconds)
    }
}

/// `p4kx catalog` was given no Archive or no output file.
#[derive(Debug, Error)]
#[error("p4kx catalog needs an Archive and --out")]
pub(crate) struct MissingCatalogArgs;

/// The output file of `p4kx catalog` ends in `.gz`, which would be written as plain JSON.
#[derive(Debug, Error)]
#[error(
    "{0} would be written as plain JSON, because gzip is not supported; use a name ending in .json.zst for a compressed catalog or .json for a plain one"
)]
pub(crate) struct GzipNotSupported(String);

/// Returns whether `error` is a command line `p4kx catalog` refuses: a missing Archive or output,
/// an export time out of range or an output file that would silently be plain JSON.
pub(crate) fn is_invalid_input(error: &anyhow::Error) -> bool {
    error.downcast_ref::<InvalidExportTime>().is_some()
        || error.downcast_ref::<MissingCatalogArgs>().is_some()
        || error.downcast_ref::<GzipNotSupported>().is_some()
}

/// Fails for an output file ending in `.gz`, ignoring case.
pub(crate) fn refuse_gzip(out: &Path) -> Result<(), GzipNotSupported> {
    if out
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
    {
        Err(GzipNotSupported(out.display().to_string()))
    } else {
        Ok(())
    }
}
