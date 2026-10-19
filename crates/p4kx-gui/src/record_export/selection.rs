use crate::{RecordNodeId, RecordNodeKind, RecordTree};

/// Returns the definition positions of the Records at and below the node `id` of `tree`, in tree
/// order.
pub(crate) fn records_below(tree: &RecordTree, id: RecordNodeId) -> Vec<u32> {
    let mut records = Vec::new();
    let mut pending = vec![id];
    while let Some(node) = pending.pop() {
        let node = tree.node(node);
        if let RecordNodeKind::Record(record) = node.kind() {
            records.push(record.0);
        }
        pending.extend(node.children().iter().rev().copied());
    }
    records
}
