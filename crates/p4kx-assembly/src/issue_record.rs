// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::IssueKind;

/// Something an Assembly could not do for a part, found while building it.
///
/// A part that cannot be resolved is left out or kept without geometry, and the rest of the
/// Assembly is built; the issue says which part and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) kind: IssueKind,
}

impl Issue {
    /// Returns the path of the part's port from the root, port names joined by `/`; empty for
    /// the root Entity.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the class name of the part's Entity, or what the Loadout names for an attachment
    /// that cannot be resolved.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns what went wrong.
    #[must_use]
    pub const fn kind(&self) -> &IssueKind {
        &self.kind
    }
}
