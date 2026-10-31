// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Deserialize;

/// The longest `folder` or `filter` argument accepted, in bytes.
pub(crate) const MAX_PATH_LEN: usize = 1024;

/// The arguments of `list_channels`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChannelsParams {
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}

/// The arguments of `archive_info`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArchiveParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
}

/// The arguments of `list_entries`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The folder to list, e.g. `Data/Libs`; omit it for the top level. A container is opened
    /// with `::`: `Data/Ships/a.socpak::` lists its top level, `a.socpak::b.socpak::folder` a
    /// folder of a container inside it.
    pub(crate) folder: Option<String>,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}

/// The arguments of `search_entries`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The Filter, e.g. `*.xml`, `Data/Libs/*` or `defaultProfile`; at most 1024 bytes with at
    /// most 32 `*` and `?`, otherwise the call is an error.
    pub(crate) filter: String,
    /// Searches inside this container instead of the Archive: the Entry path of a container,
    /// or several joined by `::` for containers inside containers.
    pub(crate) container: Option<String>,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}

/// The arguments of `read_entry_text`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The Entry path, e.g. `Data/Libs/Config/defaultProfile.xml`; case and separator kind do not
    /// matter. An Entry inside a container is `outer.socpak::inner.socpak::path`.
    pub(crate) path: String,
    /// The byte offset in the UTF-8 text to start at, 0 by default; pass `next_offset` to continue.
    pub(crate) offset: Option<u64>,
    /// The most bytes of UTF-8 text to return, 32768 by default, at least 4 and at most 262144.
    pub(crate) max_bytes: Option<u32>,
}

/// The arguments of `read_entry_bytes`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BytesParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The Entry path, e.g. `Data/Objects/ship.cgf`; case and separator kind do not matter. An
    /// Entry inside a container is `outer.socpak::inner.socpak::path`.
    pub(crate) path: String,
    /// The byte offset in the Entry data to start at, 0 by default; pass `next_offset` to continue.
    pub(crate) offset: Option<u64>,
    /// The most bytes to return before base64 encoding, 16384 by default, at least 1 and at most
    /// 196608.
    pub(crate) max_bytes: Option<u32>,
}
