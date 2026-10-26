// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::ReplacementMismatch;

use crate::model_info::printable;

/// A replacement that was applied although its Entity does not fit the port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortMismatch {
    /// The path of the port, port names joined by `/`.
    pub path: String,
    /// The class name of the Entity.
    pub entity: String,
    /// What does not fit.
    pub reason: String,
}

impl PortMismatch {
    pub(crate) fn all(mismatches: &[ReplacementMismatch]) -> Vec<Self> {
        mismatches
            .iter()
            .map(|mismatch| Self {
                path: printable(mismatch.path()),
                entity: printable(mismatch.entity()),
                reason: mismatch.reason().to_string(),
            })
            .collect()
    }
}
