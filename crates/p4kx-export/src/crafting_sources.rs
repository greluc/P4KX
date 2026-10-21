use std::collections::HashSet;

use p4kx_extraction::{Guid, RecordTree, StructValue, Value};

use crate::crafting_types::RewardSource;
use crate::record_fields::{guid_value, number, text};

const MAX_DEPTH: usize = 128;

/// The nearest enclosing identity of a struct: the `id` and `debugName` it or an ancestor has.
#[derive(Clone, Copy, Default)]
struct Scope<'f> {
    id: Option<Guid>,
    name: Option<&'f str>,
}

/// Appends to `out` every struct of `tree` with a Reference to one of the `pools`, with the
/// chance the struct states.
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
    let scope = Scope {
        id: guid_value(Some(node), "id").or(parent.id),
        name: text(Some(node), "debugName").or(parent.name),
    };
    let next = depth.saturating_add(1);
    for field in node.fields() {
        match field.value() {
            Value::Reference(reference)
                if reference.location().is_none() && pools.contains(&reference.guid()) =>
            {
                out.push(RewardSource {
                    record: tree.guid(),
                    record_path: tree.record_path(),
                    contract_id: scope.id,
                    contract_name: scope.name,
                    pool: Some(reference.guid()),
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
