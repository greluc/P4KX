// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::RecordTree;

use crate::contract_crafter::{Crafter, CrafterQueue};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, component, integer, structs, text};

const COMPONENT: &str = "CrafterComponentParams";

/// Returns the crafting station the entity Record tree `tree` describes, `None` for an entity
/// without a `CrafterComponentParams`.
pub(crate) fn crafter<'f>(tree: &RecordTree<'f>) -> Option<Crafter<'f>> {
    let params = component(tree, COMPONENT)?;
    Some(Crafter {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        max_jobs_in_history: integer(Some(params), "maxJobsInHistory"),
        queues: structs(Some(params), "queues")
            .map(|queue| {
                let queue = Some(queue);
                CrafterQueue {
                    name: text(queue, "debugName"),
                    max_jobs_in_progress: integer(queue, "maxJobsInProgress"),
                    max_jobs_waiting: integer(queue, "maxJobsWaiting"),
                }
            })
            .collect(),
        host_guids: Vec::new(),
    })
}
