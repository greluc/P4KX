// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_export::StoreSlugSummary;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct StoreSlugsJson<'a> {
    slugs: usize,
    unmapped: usize,
    mapped: usize,
    resolved: usize,
    retired: usize,
    exported: usize,
    stale: &'a [String],
}

impl<'a> From<&'a StoreSlugSummary> for StoreSlugsJson<'a> {
    fn from(summary: &'a StoreSlugSummary) -> Self {
        Self {
            slugs: summary.slugs,
            unmapped: summary.unmapped,
            mapped: summary.mapped,
            resolved: summary.resolved,
            retired: summary.retired,
            exported: summary.exported,
            stale: &summary.stale,
        }
    }
}
