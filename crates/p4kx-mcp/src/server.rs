use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{Json, ServerHandler, tool, tool_handler, tool_router};
use tokio::sync::Semaphore;

use crate::archives::{Archives, Opened};
use crate::entries;
use crate::error::{ToolError, capped};
use crate::locate::Locations;
use crate::page::Page;
use crate::params::{ArchiveParams, ChannelsParams, ListParams, MAX_PATH_LEN, SearchParams};
use crate::shapes::{ArchiveInfo, ChannelShape, ChannelsResult, FolderResult, SearchResult};

const WHOLE_TEXT_DECODERS: usize = 4;

const INSTRUCTIONS: &str = "Read-only access to the Data.p4k Archives of Star Citizen. \
An Archive is named by a Channel name such as LIVE (see list_channels) or by the absolute path \
of a .p4k file; without one the tools use the Archive the server was started with. Listings \
page: pass next_cursor back as cursor for the next page. Entry content is read in ranges: pass \
next_offset back as offset for the next range. The DataForge tools (get_record, search_records, \
list_structs, list_enums, run_query) read the Archive's DataForge, its typed Records; list_structs \
gives the type and field names a Query uses.";

/// The MCP server: read-only tools over the detected Channels and their Archives.
#[derive(Debug, Clone)]
pub(crate) struct Server {
    state: Arc<State>,
    workers: Arc<Semaphore>,
    whole_text: Arc<Semaphore>,
    tool_router: ToolRouter<Self>,
}

/// What every tool call reads: where Channels are found, the default Archive and the open
/// Archives.
#[derive(Debug)]
pub(crate) struct State {
    locations: Locations,
    default_archive: Option<PathBuf>,
    archives: Archives,
}

impl Server {
    /// Creates the server; `default_archive` serves calls that name no Archive.
    pub(crate) fn new(locations: Locations, default_archive: Option<PathBuf>) -> Self {
        Self {
            state: Arc::new(State {
                locations,
                default_archive,
                archives: Archives::default(),
            }),
            workers: Arc::new(Semaphore::new(
                std::thread::available_parallelism().map_or(1, NonZeroUsize::get),
            )),
            whole_text: Arc::new(Semaphore::new(WHOLE_TEXT_DECODERS)),
            tool_router: Self::tool_router()
                + Self::content_router()
                + Self::record_router()
                + Self::query_router(),
        }
    }

    /// Returns the permits for decoding an Entry as a whole, at most four at a time.
    pub(crate) fn whole_text(&self) -> Arc<Semaphore> {
        Arc::clone(&self.whole_text)
    }

    /// Reads the DataForge of the Archive `archive` names if that has not happened yet. It waits
    /// on the blocking pool without a worker permit, so calls waiting for the read do not keep
    /// the workers from unrelated tools.
    pub(crate) async fn preload(&self, archive: Option<String>) -> Result<(), String> {
        let state = Arc::clone(&self.state);
        let outcome = tokio::task::spawn_blocking(move || {
            state.archive(archive.as_deref())?.load_dataforge()
        })
        .await;
        match outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.message()),
            Err(_) => Err(ToolError::Stopped.message()),
        }
    }
    /// Runs `work` on the blocking pool, at most one call per CPU at a time.
    pub(crate) async fn run<T, F>(&self, work: F) -> Result<Json<T>, String>
    where
        T: Send + 'static,
        F: FnOnce(&State) -> Result<T, ToolError> + Send + 'static,
    {
        let Ok(permit) = Arc::clone(&self.workers).acquire_owned().await else {
            return Err(ToolError::Stopped.message());
        };
        let state = Arc::clone(&self.state);
        let outcome = tokio::task::spawn_blocking(move || {
            let result = work(&state);
            drop(permit);
            result
        })
        .await;
        match outcome {
            Ok(Ok(value)) => Ok(Json(value)),
            Ok(Err(error)) => Err(error.message()),
            Err(_) => Err(ToolError::Stopped.message()),
        }
    }
}

impl State {
    /// Returns the Archive `archive` names, or the default Archive when it is `None`.
    pub(crate) fn archive(&self, archive: Option<&str>) -> Result<Arc<Opened>, ToolError> {
        let path = match archive {
            Some(text) => self.locations.resolve(text)?,
            None => self.default_archive.clone().ok_or(ToolError::NoArchive)?,
        };
        self.archives.get(&path)
    }
}

#[tool_router(vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "List the installed Channels of Star Citizen (LIVE, PTU, ...) with their Archive path and Build.",
        annotations(
            title = "List Channels",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_channels(
        &self,
        Parameters(params): Parameters<ChannelsParams>,
    ) -> Result<Json<ChannelsResult>, String> {
        self.run(move |state| {
            let page = Page::new(&"list_channels", params.cursor.as_deref(), params.limit)?;
            let channels = state.locations.channels()?;
            Ok(ChannelsResult {
                total: channels.len(),
                channels: page
                    .slice(&channels)
                    .iter()
                    .map(ChannelShape::from)
                    .collect(),
                next_cursor: page.next_cursor(channels.len())?,
            })
        })
        .await
    }

    #[tool(
        description = "Summarise an Archive: its Channel, Build, number of Entries, encrypted Entries and sizes.",
        annotations(
            title = "Archive info",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn archive_info(
        &self,
        Parameters(params): Parameters<ArchiveParams>,
    ) -> Result<Json<ArchiveInfo>, String> {
        self.run(move |state| {
            let opened = state.archive(params.archive.as_deref())?;
            Ok(entries::info(opened.archive()))
        })
        .await
    }

    #[tool(
        description = "List one folder of an Archive: its subfolders, then its Entries. Folder paths ignore case and separator kind; omit folder for the top level.",
        annotations(
            title = "List Entries",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_entries(
        &self,
        Parameters(params): Parameters<ListParams>,
    ) -> Result<Json<FolderResult>, String> {
        self.run(move |state| {
            let folder = capped(
                "folder",
                params.folder.as_deref().unwrap_or_default(),
                MAX_PATH_LEN,
            )?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("list_entries", opened.version(), folder);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            entries::list(&opened, folder, page)
        })
        .await
    }

    #[tool(
        description = "Search the Entries of an Archive by path. A filter with * or ? is a glob over the whole path (* also matches separators); any other filter matches anywhere in the path. Case and separator kind do not matter.",
        annotations(
            title = "Search Entries",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_entries(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<Json<SearchResult>, String> {
        self.run(move |state| {
            let filter = capped("filter", &params.filter, MAX_PATH_LEN)?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("search_entries", opened.version(), filter);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            entries::search(opened.archive(), filter, page)
        })
        .await
    }
}

#[expect(
    clippy::unused_async_trait_impl,
    reason = "the SDK macro generates async handlers that need no await"
)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
                    .with_title("P4K-Extractor"),
            )
            .with_instructions(INSTRUCTIONS)
    }
}
