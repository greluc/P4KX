// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{DataForge, Filter, Guid, JsonLayout, Record, RecordJson, TreeBuilder};
use rayon::prelude::*;
use serde_json::value::RawValue;

use crate::budget::{Budget, Capped, MAX_RESPONSE_LEN, fill_within, json_len};
use crate::error::ToolError;
use crate::forge_shapes::{RecordResult, RecordSearchResult, RecordShape};
use crate::page::Page;

/// How `get_record` names its Records.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Selector {
    /// The Record with this GUID.
    Guid(String),
    /// The Records with this record path.
    Path(String),
}

impl Selector {
    /// Returns the selector of a call that passes exactly one of `guid` and `path`.
    pub(crate) fn new(guid: Option<String>, path: Option<String>) -> Result<Self, ToolError> {
        match (guid, path) {
            (Some(guid), None) => Ok(Self::Guid(guid)),
            (None, Some(path)) => Ok(Self::Path(path)),
            _ => Err(ToolError::RecordSelector),
        }
    }
}

/// Returns one page of the Records `selector` names as Record JSON, as many as fit the response
/// cap.
pub(crate) fn get(
    forge: &DataForge<'_>,
    archive: String,
    selector: &Selector,
    page: Page,
) -> Result<RecordResult, ToolError> {
    let records = select(forge, selector)?;
    let total = records.len();
    let mut budget = Budget::new();
    budget.take(json_len(&(&archive, total)).saturating_add(128));
    let mut builder = TreeBuilder::new();
    let mut writer = RecordJson::new(JsonLayout::Compact);
    let mut taken = Vec::new();
    for record in records.iter().skip(page.offset()).take(page.limit()) {
        let tree = builder.build(record)?;
        let mut out = Capped::new(budget.left());
        let fits = writer.write(&tree, &mut out).is_ok();
        let bytes = out.into_bytes();
        if !fits || !budget.take(bytes.len()) {
            if taken.is_empty() {
                return Err(ToolError::TooLargeItem {
                    what: format!("Record {}", record.name()?),
                    limit: MAX_RESPONSE_LEN,
                    hint: "run_query selects single values of it",
                });
            }
            break;
        }
        taken.push(RawValue::from_string(String::from_utf8(bytes)?)?);
    }
    let next_cursor = page.next_cursor_after(taken.len(), total)?;
    Ok(RecordResult {
        archive,
        total,
        records: taken,
        next_cursor,
    })
}

fn select<'f>(forge: &'f DataForge<'f>, selector: &Selector) -> Result<Vec<Record<'f>>, ToolError> {
    match selector {
        Selector::Guid(text) => {
            let guid: Guid = text
                .trim()
                .parse()
                .map_err(|_| ToolError::BadGuid(text.clone()))?;
            let record = forge
                .record_by_guid(guid)
                .ok_or_else(|| ToolError::UnknownGuid(guid.to_string()))?;
            Ok(vec![record])
        }
        Selector::Path(path) => {
            let records: Vec<Record<'f>> = forge.records_at_path(path).collect();
            if records.is_empty() {
                return Err(ToolError::UnknownRecordPath(path.clone()));
            }
            Ok(records)
        }
    }
}

/// Returns one page of the Records whose record path matches `path` and whose type name
/// matches `type_name`, both Filters, in Record order.
pub(crate) fn search(
    forge: &DataForge<'_>,
    archive: String,
    (path, type_name): (Option<&str>, Option<&str>),
    page: Page,
) -> Result<RecordSearchResult, ToolError> {
    let path = path.map(Filter::parse).transpose()?;
    let type_name = type_name.map(Filter::parse).transpose()?;
    let matches: Vec<u32> = forge
        .path_order()
        .par_iter()
        .copied()
        .filter(|&index| {
            forge.record(index).is_some_and(|record| {
                path.as_ref().is_none_or(|filter| {
                    record.record_path().is_ok_and(|text| filter.matches(text))
                }) && type_name
                    .as_ref()
                    .is_none_or(|filter| record.type_name().is_ok_and(|text| filter.matches(text)))
            })
        })
        .collect();
    let items = page
        .slice(&matches)
        .iter()
        .filter_map(|&index| forge.record(index))
        .map(|record| shape(&record));
    let mut result = RecordSearchResult {
        archive,
        total: matches.len(),
        records: Vec::new(),
        next_cursor: None,
    };
    let (records, next_cursor) = fill_within(
        page,
        matches.len(),
        json_len(&result),
        items,
        |record: &RecordShape| format!("Record {}", record.name),
    )?;
    result.records = records;
    result.next_cursor = next_cursor;
    Ok(result)
}

fn shape(record: &Record<'_>) -> Result<RecordShape, ToolError> {
    Ok(RecordShape {
        guid: record.guid().to_string(),
        name: record.name()?.to_owned(),
        type_name: record.type_name()?.to_owned(),
        path: record.record_path()?.to_owned(),
    })
}
