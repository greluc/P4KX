// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Deserialize;

/// The longest `guid` argument accepted, in bytes.
pub(crate) const MAX_GUID_LEN: usize = 64;
/// The longest `name` argument accepted, in bytes.
pub(crate) const MAX_NAME_LEN: usize = 256;

/// The arguments of `get_record`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file or of a DataForge
    /// file such as `Game2.dcb`; omit it for the Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The GUID of the Record, e.g. `cbee34fd-6441-4000-b99f-924f7f26e19f`; pass it or `path`.
    pub(crate) guid: Option<String>,
    /// The record path, e.g. `libs/foundry/records/entities/spaceships/aegs_avenger`; case,
    /// separator kind and a trailing `.xml` do not matter. Pass it or `guid`.
    pub(crate) path: Option<String>,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
}

/// The arguments of `search_records`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordSearchParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file or of a DataForge
    /// file such as `Game2.dcb`; omit it for the Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// A Filter over the record path, e.g. `*/spaceships/*` or `aegs_avenger`; omit it for every
    /// record path.
    pub(crate) path: Option<String>,
    /// A Filter over the type name, e.g. `EntityClassDefinition` or `*Weapon*`; omit it for every
    /// type.
    #[serde(rename = "type")]
    pub(crate) type_name: Option<String>,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}

/// The arguments of `list_structs` and `list_enums`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DefinitionParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file or of a DataForge
    /// file such as `Game2.dcb`; omit it for the Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// A Filter over the name, e.g. `Weapon` or `SCItem*Params`; omit it for every definition.
    pub(crate) name: Option<String>,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}

/// The arguments of `run_query`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct QueryParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file or of a DataForge
    /// file such as `Game2.dcb`; omit it for the Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The Query, e.g. `from EntityClassDefinition at "*/spaceships/*" select Components[VehicleComponentParams].vehicleName`.
    pub(crate) query: String,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size in rows, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}
