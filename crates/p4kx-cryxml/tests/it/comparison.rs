// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The child table against the tree the parent indices give: a diverging table is reported, and
//! the tree stays the one of the parent indices.

use std::error::Error;

use p4kx_cryxml::{TreeComparison, compare_trees, parse};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

type TestResult = Result<(), Box<dyn Error>>;

const NODE_TABLE: usize = 44;
const NODE_LEN: usize = 28;
const FIRST_CHILD_FIELD: usize = 20;
const CHILD_TABLE_FIELD: usize = 28;

fn two_children(second: &str) -> Vec<u8> {
    TestElement::new("root")
        .child(TestElement::new("item").attribute("n", "1"))
        .child(TestElement::new("item").attribute("n", second))
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) -> TestResult {
    bytes
        .get_mut(offset..offset + 4)
        .ok_or("offset outside the sample")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn child_slot(bytes: &[u8], slot: usize) -> Result<usize, Box<dyn Error>> {
    let table: [u8; 4] = bytes
        .get(CHILD_TABLE_FIELD..CHILD_TABLE_FIELD + 4)
        .ok_or("no header")?
        .try_into()?;
    let position = u32::from_le_bytes(table);
    Ok(position as usize + slot * 4)
}

fn set_child(bytes: &mut [u8], slot: usize, node: u32) -> TestResult {
    let offset = child_slot(bytes, slot)?;
    put_u32(bytes, offset, node)
}

fn comparison(bytes: &[u8]) -> Result<TreeComparison, Box<dyn Error>> {
    Ok(compare_trees(bytes)?)
}

#[test]
fn a_consistent_child_table_agrees() -> TestResult {
    let result = comparison(&two_children("1"))?;

    assert!(result.child_table_agrees(), "{result:?}");
    assert_eq!(result.nodes, 3, "nodes");
    assert_eq!(result.child_table_entries, 2, "child table entries");
    assert_eq!(result.attributes_not_sequential, 0, "attributes");
    Ok(())
}

#[test]
fn a_child_listed_again_for_an_equal_sibling_is_an_equal_copy() -> TestResult {
    let mut bytes = two_children("1");
    set_child(&mut bytes, 1, 1)?;

    let result = comparison(&bytes)?;

    assert_eq!(result.children_naming_equal_copies, 1, "{result:?}");
    assert_eq!(result.children_naming_other_nodes, 0, "{result:?}");
    assert!(!result.child_table_agrees(), "disagrees");
    Ok(())
}

#[test]
fn a_child_listed_again_for_a_different_sibling_names_other_nodes() -> TestResult {
    let mut bytes = two_children("2");
    set_child(&mut bytes, 1, 1)?;

    let result = comparison(&bytes)?;

    assert_eq!(result.children_naming_other_nodes, 1, "{result:?}");
    assert_eq!(result.children_naming_equal_copies, 0, "{result:?}");
    let document = parse(&bytes)?;
    let values: Vec<_> = document
        .root()
        .children()
        .filter_map(|child| child.attribute("n"))
        .collect();
    assert_eq!(
        values,
        ["1", "2"],
        "the tree is the one of the parent indices"
    );
    Ok(())
}

#[test]
fn a_child_table_in_another_order_is_reported() -> TestResult {
    let mut bytes = two_children("2");
    set_child(&mut bytes, 0, 2)?;
    set_child(&mut bytes, 1, 1)?;

    let result = comparison(&bytes)?;

    assert_eq!(result.children_in_other_order, 1, "{result:?}");
    Ok(())
}

#[test]
fn a_child_range_outside_the_table_is_reported() -> TestResult {
    let mut bytes = two_children("1");
    put_u32(&mut bytes, NODE_TABLE + FIRST_CHILD_FIELD, 99)?;

    let result = comparison(&bytes)?;

    assert_eq!(result.child_range_outside_table, 1, "{result:?}");
    Ok(())
}

#[test]
fn attributes_that_are_not_sequential_are_reported() -> TestResult {
    let mut bytes = two_children("1");
    put_u32(&mut bytes, NODE_TABLE + NODE_LEN + 16, 1)?;

    let result = comparison(&bytes)?;

    assert_eq!(
        result.attributes_not_sequential, 2,
        "the moved node and the one after it: {result:?}"
    );
    Ok(())
}

#[test]
fn comparing_a_truncated_file_is_an_error() {
    let bytes = two_children("1");
    for length in 0..bytes.len() {
        let result = compare_trees(bytes.get(..length).unwrap_or_default());
        assert!(result.is_err(), "truncated to {length} bytes");
    }
}
