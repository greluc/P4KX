// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// What a catalog holds of the curated RSI store slugs (ADR-0117).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoreSlugSummary {
    /// The slugs in the data file.
    pub slugs: usize,
    /// The slugs with no ship mapped yet.
    pub unmapped: usize,
    /// The slugs with a mapped ship, retired ones included.
    pub mapped: usize,
    /// The mapped, not retired slugs whose ship the catalog has.
    pub resolved: usize,
    /// The slugs marked retired.
    pub retired: usize,
    /// The slugs the catalog lists: the resolved ones and, without a Filter, the retired ones.
    pub exported: usize,
    /// The slugs whose mapping no longer holds: the GUID is no ship of the catalog, the ship has
    /// another class name, or a retired slug's ship exists again. Always empty with a Filter and
    /// when [`stale_check_skipped`](Self::stale_check_skipped).
    pub stale: Vec<String>,
    /// Whether the stale check was skipped because slugs are mapped but none resolves, as for an
    /// Archive that is not the game's: every mapping would be stale.
    pub stale_check_skipped: bool,
}
