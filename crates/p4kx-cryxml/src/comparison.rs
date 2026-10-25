// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::document::{Document, parse};
use crate::error::CryXmlError;
use crate::format::Signature;
use crate::header::{Header, Reader};

/// How the child table and the attribute positions of a `CryXml` file relate to the tree its
/// parent indices give.
///
/// Counts are of nodes. A node whose child range lies outside the child table is counted only in
/// `child_range_outside_table`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeComparison {
    /// The number of nodes.
    pub nodes: u32,
    /// The number of entries in the child table.
    pub child_table_entries: u32,
    /// Nodes whose child range lies outside the child table.
    pub child_range_outside_table: u32,
    /// Nodes whose child table lists the children their parent indices give, in another order.
    pub children_in_other_order: u32,
    /// Nodes whose child table lists an element again where the parent indices give an equal one:
    /// same names, contents, attributes and shape of the whole subtree.
    pub children_naming_equal_copies: u32,
    /// Nodes whose child table lists a node whose parent index names another node, or misses one
    /// whose parent index names this node, and which `children_naming_equal_copies` does not cover.
    pub children_naming_other_nodes: u32,
    /// Whether the tree read from the child table, starting at the root, has the same names,
    /// contents, attributes and shape as the tree the parent indices give. False too when the
    /// child table loops, lies outside itself or needs more steps than the work bound allows.
    pub child_table_tree_equal: bool,
    /// Nodes whose first attribute is not where the attributes of the nodes before it end.
    pub attributes_not_sequential: u32,
}

impl TreeComparison {
    /// Returns whether the child table gives exactly the tree the parent indices give.
    #[must_use]
    pub const fn child_table_agrees(&self) -> bool {
        self.child_range_outside_table == 0
            && self.children_in_other_order == 0
            && self.children_naming_equal_copies == 0
            && self.children_naming_other_nodes == 0
    }
}

/// Compares the child table of `bytes` with the tree its parent indices give.
///
/// # Errors
///
/// Returns the [`CryXmlError`] [`parse`] returns for `bytes`.
pub fn compare_trees(bytes: &[u8]) -> Result<TreeComparison, CryXmlError> {
    let document = parse(bytes)?;
    let signature = Signature::of(bytes).ok_or(CryXmlError::NotCryXml)?;
    let header = Header::read(bytes, signature)?;
    let table = child_table(&header)?;
    let mut comparison = TreeComparison {
        nodes: u32::try_from(document.nodes.len()).unwrap_or(u32::MAX),
        child_table_entries: u32::try_from(table.len()).unwrap_or(u32::MAX),
        ..TreeComparison::default()
    };
    let mut reader = Reader::new(header.nodes, header.byte_order);
    let mut next_attribute = 0_u32;
    let mut ranges: Vec<(u32, u16)> = Vec::with_capacity(document.nodes.len());
    let mut budget = document
        .nodes
        .len()
        .saturating_mul(STEPS_PER_NODE)
        .saturating_add(MIN_STEPS);
    for index in 0..document.nodes.len() {
        let _name = reader.u32()?;
        let _content = reader.u32()?;
        let attribute_count = reader.u16()?;
        let child_count = reader.u16()?;
        let _parent = reader.u32()?;
        let first_attribute = reader.u32()?;
        let first_child = reader.u32()?;
        let _reserved = reader.u32()?;

        if first_attribute != next_attribute {
            comparison.attributes_not_sequential += 1;
        }
        next_attribute = first_attribute.saturating_add(u32::from(attribute_count));

        ranges.push((first_child, child_count));
        let declared = usize::try_from(first_child)
            .ok()
            .and_then(|first| table.get(first..first.checked_add(usize::from(child_count))?));
        let Some(declared) = declared else {
            comparison.child_range_outside_table += 1;
            continue;
        };
        classify(&document, index, declared, &mut budget, &mut comparison);
    }
    let mut walk = document.nodes.len().saturating_add(1);
    comparison.child_table_tree_equal =
        child_table_tree_equal(&document, &table, &ranges, &mut walk);
    Ok(comparison)
}

/// Counts how the `declared` children of node `index` differ from the ones the parent indices
/// give. A list of another length is classified without sorting, so the sorting of all nodes
/// together stays within the node count.
fn classify(
    document: &Document<'_>,
    index: usize,
    declared: &[u32],
    budget: &mut usize,
    comparison: &mut TreeComparison,
) {
    let expected = derived_children(document, index);
    if declared == expected {
        return;
    }
    if declared.len() != expected.len() {
        comparison.children_naming_other_nodes += 1;
        return;
    }
    let mut listed = declared.to_vec();
    listed.sort_unstable();
    let mut sorted = expected.to_vec();
    sorted.sort_unstable();
    if listed == sorted {
        comparison.children_in_other_order += 1;
    } else if only_equal_copies(document, &listed, &sorted, budget) {
        comparison.children_naming_equal_copies += 1;
    } else {
        comparison.children_naming_other_nodes += 1;
    }
}

/// Walks the tree the child table gives from the root together with the tree the parent indices
/// give, and returns whether every element has the same name, content, attributes and shape.
/// A child table that loops, lies outside itself or needs more than `budget` steps is not equal.
fn child_table_tree_equal(
    document: &Document<'_>,
    table: &[u32],
    ranges: &[(u32, u16)],
    budget: &mut usize,
) -> bool {
    let mut stack = vec![(0_u32, 0_u32)];
    while let Some((listed, parent_tree)) = stack.pop() {
        let Some(left_over) = budget.checked_sub(1) else {
            return false;
        };
        *budget = left_over;
        let expected = derived_children(document, parent_tree as usize);
        let declared = ranges.get(listed as usize).and_then(|&(first, count)| {
            let first = usize::try_from(first).ok()?;
            table.get(first..first.checked_add(usize::from(count))?)
        });
        let Some(declared) = declared else {
            return false;
        };
        if declared.len() != expected.len() || !equal_elements(document, listed, parent_tree) {
            return false;
        }
        stack.extend(declared.iter().copied().zip(expected.iter().copied()));
    }
    true
}

const MAX_COPIES: usize = 64;
const STEPS_PER_NODE: usize = 4;
const MIN_STEPS: usize = 4096;

/// Returns whether every node the sorted `declared` list names too often or wrongly stands in for
/// a node of the sorted `expected` list whose subtree is equal: same names, contents, attributes
/// and shape. Every comparison step is taken from `budget`; once it is spent the answer is no.
fn only_equal_copies(
    document: &Document<'_>,
    declared: &[u32],
    expected: &[u32],
    budget: &mut usize,
) -> bool {
    let (extra, missing) = difference(declared, expected);
    if extra.len() != missing.len() || missing.len() > MAX_COPIES {
        return false;
    }
    let mut used = vec![false; extra.len()];
    missing.iter().all(|&wanted| {
        extra.iter().zip(used.iter_mut()).any(|(&listed, taken)| {
            let equal = !*taken && equal_subtrees(document, listed, wanted, budget);
            *taken |= equal;
            equal
        })
    })
}

fn difference(declared: &[u32], expected: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let (mut extra, mut missing) = (Vec::new(), Vec::new());
    let (mut left, mut right) = (declared.iter().peekable(), expected.iter().peekable());
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (Some(a), Some(b)) if a == b => {
                left.next();
                right.next();
            }
            (Some(a), Some(b)) if a < b => {
                extra.push(*a);
                left.next();
            }
            (Some(a), None) => {
                extra.push(*a);
                left.next();
            }
            (Some(_) | None, Some(b)) => {
                missing.push(*b);
                right.next();
            }
            (None, None) => return (extra, missing),
        }
    }
}

fn equal_subtrees(document: &Document<'_>, left: u32, right: u32, budget: &mut usize) -> bool {
    let mut stack = vec![(left, right)];
    while let Some((a, b)) = stack.pop() {
        let Some(left_over) = budget.checked_sub(1) else {
            return false;
        };
        *budget = left_over;
        let (a_children, b_children) = (
            derived_children(document, a as usize),
            derived_children(document, b as usize),
        );
        if a_children.len() != b_children.len() || !equal_elements(document, a, b) {
            return false;
        }
        stack.extend(a_children.iter().copied().zip(b_children.iter().copied()));
    }
    true
}

fn equal_elements(document: &Document<'_>, left: u32, right: u32) -> bool {
    let (Some(a), Some(b)) = (
        document.nodes.get(left as usize),
        document.nodes.get(right as usize),
    ) else {
        return false;
    };
    let text = |span: crate::strings::Span| span.get(&document.text);
    let attributes = |node: &crate::document::Node| {
        let first = node.first_attribute as usize;
        document
            .attributes
            .get(first..first.checked_add(usize::from(node.attribute_count))?)
    };
    let (Some(a_attributes), Some(b_attributes)) = (attributes(a), attributes(b)) else {
        return false;
    };
    text(a.name) == text(b.name)
        && text(a.content) == text(b.content)
        && a_attributes.len() == b_attributes.len()
        && a_attributes
            .iter()
            .zip(b_attributes)
            .all(|(x, y)| text(x.0) == text(y.0) && text(x.1) == text(y.1))
}

fn derived_children<'d>(document: &'d Document<'_>, index: usize) -> &'d [u32] {
    document
        .child_starts
        .get(index)
        .zip(document.child_starts.get(index.saturating_add(1)))
        .and_then(|(&start, &end)| document.children.get(start as usize..end as usize))
        .unwrap_or_default()
}

fn child_table(header: &Header<'_>) -> Result<Vec<u32>, CryXmlError> {
    let mut reader = Reader::new(header.children, header.byte_order);
    let count = header.children.len() / 4;
    let mut table = Vec::with_capacity(count);
    for _ in 0..count {
        table.push(reader.u32()?);
    }
    Ok(table)
}
