// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;

use p4kx_extraction::{Guid, Record};

/// A Record the catalog build could not read; the entry it would have produced is left out.
///
/// A ship whose loadout cannot be resolved stays in `ships` without a loadout, and its `reason`
/// starts with `loadout: `.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogFailure {
    /// The GUID of the Record.
    pub guid: Guid,
    /// The record path, or an empty text when it cannot be read.
    pub record_path: String,
    /// Why the Record cannot be read.
    pub reason: String,
}

impl CatalogFailure {
    /// Returns the failure of `record`, which cannot be read because of `error`.
    pub(crate) fn of(record: &Record<'_>, error: &dyn Error) -> Self {
        let mut reason = error.to_string();
        let mut source = error.source();
        while let Some(cause) = source {
            reason.push_str(": ");
            reason.push_str(&cause.to_string());
            source = cause.source();
        }
        Self {
            guid: record.guid(),
            record_path: record.record_path().map(str::to_owned).unwrap_or_default(),
            reason,
        }
    }

    /// Returns the failure of the loadout of `record`, which cannot be resolved because of
    /// `error`.
    pub(crate) fn of_loadout(record: &Record<'_>, error: &dyn Error) -> Self {
        let mut failure = Self::of(record, error);
        failure.reason.insert_str(0, "loadout: ");
        failure
    }
}
