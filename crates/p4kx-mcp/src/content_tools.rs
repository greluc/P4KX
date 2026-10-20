use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use tokio::runtime::Handle;

use crate::content;
use crate::error::capped;
use crate::noted::Json;
use crate::params::{BytesParams, MAX_PATH_LEN, TextParams};
use crate::server::Server;
use crate::shapes::{BytesResult, TextResult};

#[tool_router(router = content_router, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Read an Entry as text, from a byte offset of its UTF-8 form and at most max_bytes long, \
        starting and ending on whole characters. CryXml is converted to XML, UTF-16 is recognised by its \
        byte order mark, other Entries must be UTF-8; binary Entries are an error. more and next_offset \
        say whether and where the text continues.",
        annotations(
            title = "Read Entry text",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn read_entry_text(
        &self,
        Parameters(params): Parameters<TextParams>,
    ) -> Result<Json<TextResult>, String> {
        let decoders = self.whole_text();
        let runtime = Handle::current();
        self.run(move |state| {
            let path = capped("path", &params.path, MAX_PATH_LEN)?;
            let opened = state.archive(params.archive.as_deref())?;
            content::text(
                opened.archive(),
                path,
                params.offset.unwrap_or_default(),
                params.max_bytes,
                || runtime.block_on(decoders.acquire_owned()),
            )
        })
        .await
    }

    #[tool(
        description = "Read the decompressed and decrypted data of an Entry from a byte offset, at most \
        max_bytes long, as base64 (standard alphabet, padded). more and next_offset say whether and \
        where the data continues.",
        annotations(
            title = "Read Entry bytes",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn read_entry_bytes(
        &self,
        Parameters(params): Parameters<BytesParams>,
    ) -> Result<Json<BytesResult>, String> {
        self.run(move |state| {
            let path = capped("path", &params.path, MAX_PATH_LEN)?;
            let opened = state.archive(params.archive.as_deref())?;
            content::bytes(
                opened.archive(),
                path,
                params.offset.unwrap_or_default(),
                params.max_bytes,
            )
        })
        .await
    }
}
