// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, StructValue};

use crate::contract_crafter::{Crafter, CrafterQueue};
use crate::crafter_rules::{self, Issues};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, component, integer, number, structs, structure, text};

const COMPONENT: &str = "CrafterComponentParams";

/// Returns the crafting station the entity Record tree `tree` describes, `None` for an entity
/// without a `CrafterComponentParams`.
pub(crate) fn crafter<'f>(tree: &RecordTree<'f>) -> Option<Crafter<'f>> {
    let params = component(tree, COMPONENT)?;
    let definition = structure(component(tree, "SAttachableComponentParams"), "AttachDef");
    let mut issues = Issues::new();
    let modifiers = crafter_rules::modifiers(params, &mut issues);
    let queues = structs(Some(params), "queues")
        .map(|queue| read_queue(queue, &mut issues))
        .collect();
    Some(Crafter {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        r#type: text(definition, "Type"),
        sub_type: text(definition, "SubType"),
        size: integer(definition, "Size"),
        max_jobs_in_history: integer(Some(params), "maxJobsInHistory"),
        base_crafting_time: number(Some(params), "baseCraftingTime"),
        queues,
        modifiers,
        issues: issues.into_iter().collect(),
        host_guids: Vec::new(),
    })
}

fn read_queue<'f>(queue: &StructValue<'f>, issues: &mut Issues) -> CrafterQueue<'f> {
    let parent = Some(queue);
    CrafterQueue {
        name: text(parent, "debugName"),
        max_jobs_in_progress: integer(parent, "maxJobsInProgress"),
        max_jobs_waiting: integer(parent, "maxJobsWaiting"),
        blueprint_category_availability: crafter_rules::availability(queue, issues),
        process_type_availability: crafter_rules::process_types(queue, issues),
    }
}
