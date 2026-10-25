// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rmcp::ErrorData;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};
use serde::Serialize;

use crate::noted::add_notice;

/// A structured result with a PNG image as an image content block, and the notice of the Archive
/// it was read from, if the Archive has one.
pub(crate) struct Pictured<T> {
    pub(crate) value: T,
    pub(crate) png: Vec<u8>,
    pub(crate) notice: Option<String>,
}

impl<T: Serialize + 'static> IntoCallToolResult for Pictured<T> {
    fn into_call_tool_result(self) -> Result<CallToolResponse, ErrorData> {
        let value = serde_json::to_value(self.value).map_err(|error| {
            ErrorData::internal_error(format!("Failed to serialize the result: {error}"), None)
        })?;
        let mut result = CallToolResult::structured(value);
        result
            .content
            .push(ContentBlock::image(STANDARD.encode(&self.png), "image/png"));
        add_notice(&mut result, self.notice);
        Ok(result.into())
    }
}
