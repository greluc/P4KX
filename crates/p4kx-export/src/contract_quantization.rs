// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::guid_text::CatalogGuid;

/// One band of a quality quantization: every quality from `start` to `end` counts as
/// `mappedValue`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct QuantizationBand {
    /// The lowest quality of the band, 0 to 999 on LIVE.
    pub(crate) start: i64,
    /// The highest quality of the band, inclusive, 399 to 1000 on LIVE.
    pub(crate) end: i64,
    /// The quality the game uses for every quality in the band, 0 to 1000 on LIVE.
    pub(crate) mapped_value: i64,
}

/// A quality quantization: a `CraftingQualityQuantizationRecord` under
/// `crafting/qualityquantization`, the quantised quality steps of the `ResourceType`s that link to
/// it. The Record itself holds no link to a `ResourceType`; the link goes the other way, so
/// `resourceTypeGuids` is derived from the `ResourceType` Records.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct QualityQuantization<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `quantization_gold`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The bands in stored order, 8 on LIVE and the same ranges for every Record.
    pub(crate) bands: Vec<QuantizationBand>,
    /// The GUIDs of the `ResourceType` Records whose crafting data link to the Record, ordered by
    /// GUID. It equals a commodity's `subtypeGuid` only when the catalog has that commodity.
    /// Empty for a Record no `ResourceType` uses; a warning then tells.
    pub(crate) resource_type_guids: Vec<CatalogGuid>,
}
