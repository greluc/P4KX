// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::guid_text::CatalogGuid;

/// An RSI store slug and the ship it stands for.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ShipStoreSlug<'a> {
    /// The slug of the ship in the RSI store, the last segment of its pledge URL, e.g.
    /// `Aurora-Mk-I-ES`; entries are ordered by it.
    pub(crate) slug: &'a str,
    /// The ship id of the RSI store.
    pub(crate) rsi_id: u32,
    /// The ship name as the RSI store shows it.
    pub(crate) store_name: &'a str,
    /// The `className` of the ship the slug maps to, `null` for a retired slug.
    pub(crate) class_name: Option<&'a str>,
    /// The `guid` of the ship of `ships` the slug maps to, `null` for a retired slug.
    pub(crate) ship_guid: Option<CatalogGuid>,
    /// Whether the slug's ship no longer exists in the game data; the slug stays so that a
    /// pledge of it is recognised as retired.
    pub(crate) retired: bool,
}
