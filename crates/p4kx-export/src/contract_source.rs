// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;

/// Where a catalog came from.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct Source<'a> {
    /// The Channel: the name of the folder holding the Archive, `null` when there is none.
    pub(crate) channel: Option<String>,
    /// The Build from the Channel's build manifest, `null` when it is unknown.
    pub(crate) build: Option<BuildSource<'a>>,
    /// The size of the Archive file in bytes.
    pub(crate) archive_size: u64,
    /// The modification time of the Archive file, UTC in RFC 3339 form, `null` when unknown.
    pub(crate) archive_modified: Option<String>,
    /// The version of P4KX that wrote the catalog.
    pub(crate) tool_version: &'static str,
    /// When the catalog was written, UTC in RFC 3339 form.
    pub(crate) exported_at: String,
}

/// The Build of the Archive.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct BuildSource<'a> {
    /// The branch, e.g. `sc-alpha-4.4.0`.
    pub(crate) branch: &'a str,
    /// The version, e.g. `4.4.162.12345`.
    pub(crate) version: &'a str,
    /// The Perforce change number the Build was made from.
    pub(crate) change_number: &'a str,
    /// The `BuildId` of the build manifest.
    pub(crate) build_id: &'a str,
    /// The public patch version, e.g. `4.7.2`, from the RSI Launcher log line with the same
    /// change number; `null` when the log has none.
    pub(crate) public_patch: Option<&'a str>,
}
