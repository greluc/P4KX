// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::{LocalisedText, every_field_required};
use crate::guid_text::CatalogGuid;

/// A starmap object: a `StarMapObject` Record under `starmap/pu`, a star system, planet, moon,
/// station, jump point or other place, with its world position when a system container places it.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct StarmapObject<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `Stanton1_Hurston`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The name of the place, from `name` of the Record. The localisation key and the texts are
    /// game texts.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) name: LocalisedText<'a>,
    /// The type of the object: the `name` of the `StarMapObjectType` Record the Reference `type`
    /// links to when that is a plain string, else the Record name without its type; `null` for a
    /// null or dangling Reference. The schema keeps a string, as a new type is additive.
    pub(crate) r#type: Option<&'a str>,
    /// The GUID the Reference `type` links to, `null` for a null Reference.
    pub(crate) type_guid: Option<CatalogGuid>,
    /// The GUID of the object the Reference `parent` links to, `null` for a null Reference. Each
    /// is the `guid` of an entry of `starmapObjects` unless its Record is missing or left out.
    pub(crate) parent_guid: Option<CatalogGuid>,
    /// Whether the starmap hides the object, from `hideInStarmap`; `null` when the Record states
    /// none.
    pub(crate) hide_in_starmap: Option<bool>,
    /// The star system that places the object: the folder below `ObjectContainers/PU/system`,
    /// e.g. `stanton`; `null` when no system container places it.
    pub(crate) system: Option<String>,
    /// The `entityName` of the Child element that places the object, e.g. `jumppoint_stanton_pyro`;
    /// `null` when it is not placed or the Child names none.
    pub(crate) entity_name: Option<String>,
    /// The GUID of the nearest object above this one in the container hierarchy, `null` when it
    /// is not placed or hangs directly below the system. Each is a `guid` of `starmapObjects`
    /// unless its Record is missing or left out.
    pub(crate) placed_under_guid: Option<CatalogGuid>,
    /// The position `[x, y, z]` in metres in the frame of the system container, the position of
    /// each Child composed with the positions and the full rotations of the Children above it;
    /// `null` when no system container places the object.
    pub(crate) position: Option<[f64; 3]>,
}
