use rmcp::ErrorData;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};
use serde::Serialize;

/// A structured result whose text content keeps the key order of the serialised value.
///
/// The structured content is a sorted `serde_json::Value`, as `rmcp::Json` makes it; the text
/// content is the serialisation itself, so Record fields and Query columns stay in their order.
pub(crate) struct Ordered<T>(pub(crate) T);

impl<T: Serialize + 'static> IntoCallToolResult for Ordered<T> {
    fn into_call_tool_result(self) -> Result<CallToolResponse, ErrorData> {
        let fail = |error: serde_json::Error| {
            ErrorData::internal_error(format!("Failed to serialize the result: {error}"), None)
        };
        let text = serde_json::to_string(&self.0).map_err(fail)?;
        let value = serde_json::from_str(&text).map_err(fail)?;
        let mut result = CallToolResult::structured(value);
        result.content = vec![ContentBlock::text(text)];
        Ok(result.into())
    }
}
