// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::HircKind;
use p4kx_extraction::hirc_select::{LayoutCounts, select};
use p4kx_extraction::hirc_view::ObjectView;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::value::RawValue;

use crate::archives::Opened;
use crate::audio_params::SoundbankObjectsParams;
use crate::audio_wem::open_bank;
use crate::budget::{fill_within, json_len};
use crate::error::ToolError;
use crate::page::Page;

/// The `HIRC` objects of a soundbank, one page.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct SoundbankObjectsResult {
    /// The path of the Archive.
    pub(crate) archive: String,
    /// The Entry path of the soundbank.
    pub(crate) soundbank: String,
    /// The bank version.
    pub(crate) version: u32,
    /// The soundbank ID.
    pub(crate) bank_id: u32,
    /// How many objects the soundbank holds.
    pub(crate) total: usize,
    /// How many objects of the whole bank the reader decodes, does not decode, does not support
    /// in the bank's version or finds malformed.
    pub(crate) layouts: LayoutCounts,
    /// How many objects pass the filters, on this page and beyond it.
    pub(crate) matched: usize,
    /// The matching objects on this page in bank order, as `p4kx bnk objects --json` lists them.
    #[schemars(with = "Vec<ObjectView<'static>>")]
    pub(crate) objects: Vec<Box<RawValue>>,
    /// The cursor of the next page; absent on the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}

/// Returns the object type `text` names, as [`HircKind::slug`] spells it.
fn parse_kind(text: &str) -> Result<HircKind, ToolError> {
    HircKind::from_slug(text).ok_or_else(|| {
        let known: Vec<&str> = HircKind::known().map(HircKind::slug).collect();
        ToolError::Refused(format!(
            "`{text}` is no object type; the types are {}",
            known.join(", ")
        ))
    })
}

/// Returns one page of the `HIRC` objects of the soundbank at `params.soundbank` that pass the
/// filters.
pub(crate) fn soundbank_objects(
    opened: &Opened,
    params: &SoundbankObjectsParams,
    page: Page,
) -> Result<SoundbankObjectsResult, ToolError> {
    let kind = params.kind.as_deref().map(parse_kind).transpose()?;
    let bank = open_bank(opened, &params.soundbank)?;
    let soundbank = bank.soundbank();
    let objects = soundbank.objects();
    let selected = select(objects, kind, params.id, page.offset(), page.limit());
    page.check(selected.matched)?;
    let mut result = SoundbankObjectsResult {
        archive: opened.archive().path().display().to_string(),
        soundbank: bank.name().to_owned(),
        version: soundbank.header().version,
        bank_id: soundbank.header().id,
        total: objects.len(),
        layouts: LayoutCounts::of(objects),
        matched: selected.matched,
        objects: Vec::new(),
        next_cursor: None,
    };
    let items = selected.shown.iter().map(raw);
    let (listed, next_cursor) =
        fill_within(page, selected.matched, json_len(&result), items, |object| {
            format!("object {}", object_id(object))
        })?;
    result.objects = listed;
    result.next_cursor = next_cursor;
    Ok(result)
}

fn raw(view: &ObjectView<'_>) -> Result<Box<RawValue>, ToolError> {
    Ok(serde_json::value::to_raw_value(view)?)
}

/// Reads the ID back from a rendered object, for the message of an object too large to list.
fn object_id(object: &RawValue) -> String {
    serde_json::from_str::<serde_json::Value>(object.get())
        .ok()
        .and_then(|value| value.get("id").cloned())
        .map_or_else(String::new, |id| id.to_string())
}
