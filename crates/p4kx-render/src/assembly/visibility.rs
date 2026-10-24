// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::hash::BuildHasher;

use super::PartInfo;

/// Returns, per part, whether it is hidden.
///
/// A part is hidden when its port path is in `hidden`, when it belongs to the Interior and
/// `interior` says that is not shown, or when its parent is hidden. A part comes after its
/// parent, so one pass is enough.
#[must_use]
pub fn hidden_mask<S: BuildHasher>(
    parts: &[PartInfo],
    hidden: &HashSet<String, S>,
    interior: bool,
) -> Vec<bool> {
    let mut mask: Vec<bool> = Vec::with_capacity(parts.len());
    for part in parts {
        let inherited = part
            .parent()
            .and_then(|parent| mask.get(parent))
            .copied()
            .unwrap_or(false);
        let own = if part.is_interior() {
            !interior
        } else {
            !part.path().is_empty() && hidden.contains(part.path())
        };
        mask.push(inherited || own);
    }
    mask
}
