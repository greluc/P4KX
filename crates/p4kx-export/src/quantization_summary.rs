// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// What the quality quantizations of a catalog link to. Both counts are facts of the game data and
/// change nothing about the completeness of the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuantizationSummary {
    /// The quantizations of the catalog that no `ResourceType` uses; their `resourceTypeGuids`
    /// are empty.
    pub unused: usize,
    /// The links of `ResourceType` Records to a Record that is no readable quality quantization
    /// of the catalog: a Record of another type, a Record that failed and one left out as a
    /// development leftover. A Reference to a GUID no Record has reads as null and is not counted.
    pub unresolved_links: usize,
}
