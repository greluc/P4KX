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
    /// The folder to list, e.g. `Data/Libs`; omit it for the top level.
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
    /// The Filter, e.g. `*.xml`, `Data/Libs/*` or `defaultProfile`.
    pub(crate) filter: String,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}
