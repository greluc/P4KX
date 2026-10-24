// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use super::PartInfo;

/// Returns, per part, whether it is hidden: its port path is in `hidden` or its parent is hidden.
/// A part comes after its parent, so one pass is enough.
pub(crate) fn hidden_mask(parts: &[PartInfo], hidden: &HashSet<String>) -> Vec<bool> {
    let mut mask: Vec<bool> = Vec::with_capacity(parts.len());
    for part in parts {
        let inherited = part
            .parent()
            .and_then(|parent| mask.get(parent))
            .copied()
            .unwrap_or(false);
        mask.push(inherited || (!part.path().is_empty() && hidden.contains(part.path())));
    }
    mask
}
