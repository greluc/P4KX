// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use p4kx_extraction::{Guid, RecordTree, StructValue, Value};

use crate::crafting_types::RewardSource;
use crate::record_fields::{guid_value, number, text};

const MAX_DEPTH: usize = 128;

/// The nearest enclosing identity of a struct: the `id` and the `debugName` of the nearest struct
/// with an `id`, or, above any such struct, the nearest `debugName`.
#[derive(Clone, Copy, Default)]
struct Scope<'f> {
    id: Option<Guid>,
    name: Option<&'f str>,
}

impl<'f> Scope<'f> {
    /// Returns the scope inside `node`: its own `id` and `debugName` when it has an `id`, this
    /// scope when this one already has an `id`, otherwise the nearest `debugName`.
    fn within(self, node: &StructValue<'f>) -> Self {
        let name = text(Some(node), "debugName");
        match guid_value(Some(node), "id") {
            Some(id) => Self { id: Some(id), name },
            None if self.id.is_some() => self,
            None => Self {
                id: None,
                name: name.or(self.name),
            },
        }
    }
}

/// Appends to `out` every struct of `tree` with a Reference to one of the `pools`, with the
/// chance the struct states. A Reference of a Record to itself, as a pool Record to its own GUID,
/// is no reward source.
pub(crate) fn collect<'f>(
    tree: &RecordTree<'f>,
    pools: &HashSet<Guid>,
    out: &mut Vec<RewardSource<'f>>,
) {
    visit(tree, tree.root(), Scope::default(), pools, 0, out);
}

fn visit<'f>(
    tree: &RecordTree<'f>,
    node: &StructValue<'f>,
    parent: Scope<'f>,
    pools: &HashSet<Guid>,
    depth: usize,
    out: &mut Vec<RewardSource<'f>>,
) {
    if depth >= MAX_DEPTH {
        return;
    }
    let scope = parent.within(node);
    let next = depth.saturating_add(1);
    for field in node.fields() {
        match field.value() {
            Value::Reference(reference)
                if reference.location().is_none()
                    && reference.guid() != tree.guid()
                    && pools.contains(&reference.guid()) =>
            {
                out.push(RewardSource {
                    record: tree.guid(),
                    record_path: tree.record_path(),
                    contract_id: scope.id,
                    contract_name: scope.name,
                    pool: reference.guid(),
                    chance: number(Some(node), "chance"),
                });
            }
            Value::Struct(child) => visit(tree, child, scope, pools, next, out),
            Value::Array(items) => {
                for child in items.iter().filter_map(crate::blueprint::as_struct) {
                    visit(tree, child, scope, pools, next, out);
                }
            }
            _ => {}
        }
    }
}
