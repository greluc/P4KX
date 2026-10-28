// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::RecordTree;

use crate::contract_crafter::Crafter;
use crate::crafter_attach::attach;
use crate::crafter_fields::{Issues, checked_number};
use crate::crafter_queue::queues;
use crate::crafter_rules::modifiers;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, component, integer};

const COMPONENT: &str = "CrafterComponentParams";
const UNREADABLE_BASE_CRAFTING_TIME: &str = "unreadable_base_crafting_time";

/// Returns the crafting station the entity Record tree `tree` describes, `None` for an entity
/// without a `CrafterComponentParams`.
pub(crate) fn crafter<'f>(tree: &RecordTree<'f>) -> Option<Crafter<'f>> {
    let params = component(tree, COMPONENT)?;
    let mut issues = Issues::new();
    let attach = attach(tree, &mut issues);
    let base_crafting_time = checked_number(
        Some(params),
        "baseCraftingTime",
        UNREADABLE_BASE_CRAFTING_TIME,
        &mut issues,
    );
    let queues = queues(params, &mut issues);
    let modifiers = modifiers(params, &mut issues);
    Some(Crafter {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        r#type: attach.r#type,
        sub_type: attach.sub_type,
        size: attach.size,
        max_jobs_in_history: integer(Some(params), "maxJobsInHistory"),
        base_crafting_time,
        queues,
        modifiers,
        issues: issues.into_iter().collect(),
        host_guids: Vec::new(),
    })
}
