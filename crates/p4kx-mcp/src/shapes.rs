use p4kx_channels::Channel;
use p4kx_extraction::{Build, Entry};
use schemars::JsonSchema;
use serde::Serialize;

/// The detected Channels.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ChannelsResult {
    /// The number of detected Channels.
    pub(crate) total: usize,
    /// The Channels on this page.
    pub(crate) channels: Vec<ChannelShape>,
    /// The cursor of the next page; absent on the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}

/// One detected Channel.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ChannelShape {
    /// The Channel name, e.g. `LIVE`; it opens the Channel's Archive in every tool.
    pub(crate) name: String,
    /// The path of the Archive.
    pub(crate) archive: String,
    /// Where it was found: `default`, `launcher` or `manual`.
    pub(crate) source: String,
    /// The Build, absent when the manifest cannot be read.
    pub(crate) build: Option<BuildShape>,
}

impl From<&Channel> for ChannelShape {
    fn from(channel: &Channel) -> Self {
        Self {
            name: channel.name().to_owned(),
            archive: channel.archive().display().to_string(),
            source: channel.source().to_string(),
            build: channel.build().map(BuildShape::from),
        }
    }
}

/// The game version an Archive belongs to.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct BuildShape {
    /// The branch, e.g. `sc-alpha-4.3.1`.
    pub(crate) branch: String,
    /// The version, e.g. `4.3.1-live.10000001`.
    pub(crate) version: String,
    /// The change number.
    pub(crate) change_number: String,
    /// The `BuildId`.
    pub(crate) build_id: String,
}

impl From<&Build> for BuildShape {
    fn from(build: &Build) -> Self {
        Self {
            branch: build.branch().to_owned(),
            version: build.version().to_owned(),
            change_number: build.change_number().to_owned(),
            build_id: build.build_id().to_owned(),
        }
    }
}

/// The summary of one Archive.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ArchiveInfo {
    /// The path of the Archive.
    pub(crate) archive: String,
    /// The Channel name, the folder holding the Archive.
    pub(crate) channel: Option<String>,
    /// The Build, absent when the manifest cannot be read.
    pub(crate) build: Option<BuildShape>,
    /// The number of Entries.
    pub(crate) entry_count: usize,
    /// The number of encrypted Entries.
    pub(crate) encrypted_count: usize,
    /// The sum of the stored Entry sizes in bytes.
    pub(crate) compressed_size: u64,
    /// The sum of the Entry sizes in bytes.
    pub(crate) size: u64,
}

/// One Entry.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct EntryShape {
    /// The Entry path in the Archive.
    pub(crate) path: String,
    /// The compression method: `store`, `deflate`, `zstd` or `method-<number>`.
    pub(crate) compression: String,
    /// Whether the Entry is encrypted.
    pub(crate) encrypted: bool,
    /// The stored size in bytes.
    pub(crate) compressed_size: u64,
    /// The size in bytes.
    pub(crate) size: u64,
}

impl From<&Entry> for EntryShape {
    fn from(entry: &Entry) -> Self {
        Self {
            path: entry.path().to_owned(),
            compression: entry.compression().to_string(),
            encrypted: entry.is_encrypted(),
            compressed_size: entry.compressed_size(),
            size: entry.uncompressed_size(),
        }
    }
}

/// The content of one folder of an Archive: its subfolders first, then its Entries.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct FolderResult {
    /// The path of the Archive.
    pub(crate) archive: String,
    /// The folder listed; empty for the top level.
    pub(crate) folder: String,
    /// The number of subfolders and Entries in the folder.
    pub(crate) total: usize,
    /// The subfolders and Entries on this page.
    pub(crate) items: Vec<FolderItem>,
    /// The cursor of the next page; absent on the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}

/// A subfolder or an Entry of a folder.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FolderItem {
    /// A subfolder.
    Folder {
        /// The folder path in the Archive; pass it as `folder` to list it.
        path: String,
        /// The number of Entries below it, at any depth.
        entry_count: usize,
    },
    /// An Entry directly in the folder.
    Entry(EntryShape),
}

/// The Entries a Filter matched.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct SearchResult {
    /// The path of the Archive.
    pub(crate) archive: String,
    /// The Filter.
    pub(crate) filter: String,
    /// The number of matching Entries.
    pub(crate) total: usize,
    /// The matching Entries on this page, in Archive order.
    pub(crate) entries: Vec<EntryShape>,
    /// The cursor of the next page; absent on the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}
