// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{StructValue, Value};

use crate::contract_crafter::CrafterQueue;
use crate::crafter_fields::{Issues, array};
use crate::crafter_rules;
use crate::record_fields::{integer, text};

const UNREADABLE_QUEUES: &str = "unreadable_queues";
const UNREADABLE_QUEUE: &str = "unreadable_queue";

/// Returns the job queues the `queues` array of the crafter component `params` holds, in stored
/// order. An element that is no struct is kept as a queue without values and reported, so the
/// positions of the others do not move.
pub(crate) fn queues<'f>(params: &StructValue<'f>, issues: &mut Issues) -> Vec<CrafterQueue<'f>> {
    array(Some(params), "queues", UNREADABLE_QUEUES, issues)
        .iter()
        .map(|element| read(element, issues))
        .collect()
}

fn read<'f>(element: &Value<'f>, issues: &mut Issues) -> CrafterQueue<'f> {
    let Value::Struct(queue) = element else {
        issues.insert(UNREADABLE_QUEUE);
        return CrafterQueue {
            name: None,
            max_jobs_in_progress: None,
            max_jobs_waiting: None,
            blueprint_category_availability: crafter_rules::no_rule(),
            process_type_availability: Vec::new(),
        };
    };
    let parent = Some(queue);
    CrafterQueue {
        name: text(parent, "debugName"),
        max_jobs_in_progress: integer(parent, "maxJobsInProgress"),
        max_jobs_waiting: integer(parent, "maxJobsWaiting"),
        blueprint_category_availability: crafter_rules::availability(queue, issues),
        process_type_availability: crafter_rules::process_types(queue, issues),
    }
}
