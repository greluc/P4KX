// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{Assembly, IssueKind};
use thiserror::Error;

/// The main geometry of the Entity itself cannot be drawn, so nothing is written.
#[derive(Debug, Error)]
pub(crate) enum RootGeometryError {
    /// The geometry file is not in the Archive.
    #[error("{entity}: {reason}")]
    Missing { entity: String, reason: String },
    /// The geometry file, or a companion of it, cannot be read as a model.
    #[error("{entity}: {reason}")]
    Unreadable { entity: String, reason: String },
}

/// Fails when the root part of `assembly` has no model because its geometry file is missing or
/// unreadable; a part below the root that cannot be drawn is an Issue and no error.
pub(crate) fn check(assembly: &Assembly) -> Result<(), RootGeometryError> {
    if assembly.root().is_none_or(|root| root.model().is_some()) {
        return Ok(());
    }
    let failure = assembly
        .issues()
        .iter()
        .filter(|issue| issue.path().is_empty())
        .find_map(|issue| {
            let (entity, reason) = (issue.name().to_owned(), issue.kind().to_string());
            match issue.kind() {
                IssueKind::GeometryMissing { .. } => {
                    Some(RootGeometryError::Missing { entity, reason })
                }
                IssueKind::GeometryUnreadable { .. } => {
                    Some(RootGeometryError::Unreadable { entity, reason })
                }
                _ => None,
            }
        });
    failure.map_or(Ok(()), Err)
}
