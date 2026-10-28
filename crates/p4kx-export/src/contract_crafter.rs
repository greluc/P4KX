// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::guid_text::CatalogGuid;

/// A crafting station: an entity whose `Components` hold a `CrafterComponentParams`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct Crafter<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// `maxJobsInHistory`: how many finished jobs the station lists.
    pub(crate) max_jobs_in_history: Option<i64>,
    /// The job queues, from `queues` in stored order.
    pub(crate) queues: Vec<CrafterQueue<'a>>,
}

/// One job queue of a [`Crafter`].
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct CrafterQueue<'a> {
    /// `debugName`.
    pub(crate) name: Option<&'a str>,
    /// `maxJobsInProgress`: how many jobs the queue crafts at once.
    pub(crate) max_jobs_in_progress: Option<i64>,
    /// `maxJobsWaiting`: how many further jobs the queue holds.
    pub(crate) max_jobs_waiting: Option<i64>,
}
