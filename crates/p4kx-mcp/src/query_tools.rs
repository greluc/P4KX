use p4kx_extraction::query::MAX_QUERY_LEN;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool, tool_router};

use crate::error::capped;
use crate::forge_params::QueryParams;
use crate::forge_shapes::QueryResult;
use crate::page::Page;
use crate::query;
use crate::server::Server;

#[tool_router(router = query_router, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Run a Query over the Records of the Archive's DataForge, e.g. \
        `from EntityClassDefinition at \"*/spaceships/*\" select Components[VehicleComponentParams].crewSize where Components[VehicleComponentParams].crewSize >= 4`. \
        from selects a type and the types deriving from it, at a record path glob, select one column per dot path \
        ([n], [*], [TypeName] for array elements, -> to follow a Reference), where a condition. \
        Returns the columns and one page of rows in Record order; pass next_cursor back as cursor. \
        An invalid Query is an error naming its byte span. list_structs gives the type and field names.",
        annotations(
            title = "Run Query",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn run_query(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<Json<QueryResult>, String> {
        self.run(move |state| {
            capped("query", &params.query, MAX_QUERY_LEN)?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("run_query", opened.version(), &params.query);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            opened.with_dataforge(move |forge, archive| {
                query::run(forge, archive, &params.query, page)
            })
        })
        .await
    }
}
