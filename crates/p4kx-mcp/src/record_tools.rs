use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool, tool_router};

use crate::definitions;
use crate::error::capped;
use crate::forge_params::{
    DefinitionParams, MAX_GUID_LEN, MAX_NAME_LEN, RecordParams, RecordSearchParams,
};
use crate::forge_shapes::{EnumsResult, RecordResult, RecordSearchResult, StructsResult};
use crate::page::Page;
use crate::params::MAX_PATH_LEN;
use crate::records::{self, Selector};
use crate::server::Server;

#[tool_router(router = record_router, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Get a Record of the Archive's DataForge by GUID, or every Record with a record path, as Record JSON: \
        $type, $guid, $path and the fields; References are links with guid, path and type, never the linked Record. \
        Large results page: pass next_cursor back as cursor.",
        annotations(
            title = "Get Record",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_record(
        &self,
        Parameters(params): Parameters<RecordParams>,
    ) -> Result<Json<RecordResult>, String> {
        self.run(move |state| {
            if let Some(guid) = &params.guid {
                capped("guid", guid, MAX_GUID_LEN)?;
            }
            if let Some(path) = &params.path {
                capped("path", path, MAX_PATH_LEN)?;
            }
            let selector = Selector::new(params.guid, params.path)?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("get_record", opened.version(), &selector);
            let page = Page::new(&listing, params.cursor.as_deref(), None)?;
            opened
                .with_dataforge(move |forge, archive| records::get(forge, archive, &selector, page))
        })
        .await
    }

    #[tool(
        description = "Search the Records of the Archive's DataForge by record path and type name, both Filters: \
        with * or ? a glob over the whole text, otherwise a substring; case and separator kind do not matter. \
        Results are in Record order and page with a cursor.",
        annotations(
            title = "Search Records",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_records(
        &self,
        Parameters(params): Parameters<RecordSearchParams>,
    ) -> Result<Json<RecordSearchResult>, String> {
        self.run(move |state| {
            if let Some(path) = &params.path {
                capped("path", path, MAX_PATH_LEN)?;
            }
            if let Some(type_name) = &params.type_name {
                capped("type", type_name, MAX_NAME_LEN)?;
            }
            let opened = state.archive(params.archive.as_deref())?;
            let listing = (
                "search_records",
                opened.version(),
                &params.path,
                &params.type_name,
            );
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            opened.with_dataforge(move |forge, archive| {
                let filters = (params.path.as_deref(), params.type_name.as_deref());
                records::search(forge, archive, filters, page)
            })
        })
        .await
    }

    #[tool(
        description = "List the struct definitions of the Archive's DataForge, the types of Records and nested values, \
        with their parent and the fields each declares (name, data type, array, target struct or enum). \
        name is a Filter over the struct name. Paged with a cursor; use it to write correct Queries.",
        annotations(
            title = "List structs",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_structs(
        &self,
        Parameters(params): Parameters<DefinitionParams>,
    ) -> Result<Json<StructsResult>, String> {
        self.run(move |state| {
            if let Some(name) = &params.name {
                capped("name", name, MAX_NAME_LEN)?;
            }
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("list_structs", opened.version(), &params.name);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            opened.with_dataforge(move |forge, archive| {
                definitions::structs(forge, archive, params.name.as_deref(), page)
            })
        })
        .await
    }

    #[tool(
        description = "List the enum definitions of the Archive's DataForge with their values in order. \
        name is a Filter over the enum name. Paged with a cursor.",
        annotations(
            title = "List enums",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_enums(
        &self,
        Parameters(params): Parameters<DefinitionParams>,
    ) -> Result<Json<EnumsResult>, String> {
        self.run(move |state| {
            if let Some(name) = &params.name {
                capped("name", name, MAX_NAME_LEN)?;
            }
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("list_enums", opened.version(), &params.name);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            opened.with_dataforge(move |forge, archive| {
                definitions::enums(forge, archive, params.name.as_deref(), page)
            })
        })
        .await
    }
}
