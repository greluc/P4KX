// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cell::RefCell;

use rmcp::ErrorData;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};
use serde::Serialize;

thread_local! {
    static PENDING: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Forgets the notice of an earlier call on this thread.
pub(crate) fn clear() {
    PENDING.with_borrow_mut(Option::take);
}

/// Remembers `notice` for the result of the call running on this thread.
pub(crate) fn remember(notice: Option<&str>) {
    if let Some(notice) = notice {
        PENDING.with_borrow_mut(|pending| *pending = Some(notice.to_owned()));
    }
}

/// Takes the notice remembered by the call that ran on this thread.
pub(crate) fn take() -> Option<String> {
    PENDING.with_borrow_mut(Option::take)
}

/// A structured result with the notice of the Archive it was read from, if the Archive has one.
///
/// Named like `rmcp::Json` so that the tool macro derives the output schema from `T`. The notice
/// is one more text content block; the structured content stays the value itself.
pub(crate) struct Json<T> {
    pub(crate) value: T,
    pub(crate) notice: Option<String>,
}

impl<T: Serialize + 'static> IntoCallToolResult for Json<T> {
    fn into_call_tool_result(self) -> Result<CallToolResponse, ErrorData> {
        let value = serde_json::to_value(self.value).map_err(|error| {
            ErrorData::internal_error(format!("Failed to serialize the result: {error}"), None)
        })?;
        let mut result = CallToolResult::structured(value);
        add_notice(&mut result, self.notice);
        Ok(result.into())
    }
}

/// Appends `notice` as a text content block.
pub(crate) fn add_notice(result: &mut CallToolResult, notice: Option<String>) {
    if let Some(notice) = notice {
        result.content.push(ContentBlock::text(notice));
    }
}
