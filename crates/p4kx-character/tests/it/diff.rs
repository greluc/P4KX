// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `diff_chf` and `NameTable::merge` through the public API.

use std::error::Error;

use p4kx_character::{
    Chf, ChfChange, Guid, MAX_NAMES, NameTable, diff_chf, lowered_hash, name_hash,
};
use p4kx_testkit::{TestChf, TestItemPort};

type TestResult = Result<(), Box<dyn Error>>;

const SUB: &str = "materials.0x0badcafe.sub_materials.0x00005ab1";

fn changes(old: &TestChf, new: &TestChf) -> Result<Vec<String>, Box<dyn Error>> {
    let (old, new) = (Chf::read(&old.to_bytes())?, Chf::read(&new.to_bytes())?);
    Ok(diff_chf(&old, &new, &NameTable::builtin())
        .iter()
        .map(render)
        .collect())
}

fn render(change: &ChfChange) -> String {
    match (&change.old, &change.new) {
        (Some(old), Some(new)) => format!("{}: {old} -> {new}", change.path),
        (Some(old), None) => format!("{}: removed ({old})", change.path),
        (None, Some(new)) => format!("{}: added ({new})", change.path),
        (None, None) => change.path.clone(),
    }
}

fn guid(byte: u8) -> String {
    Guid::from_stored([byte; 16]).to_string()
}

const fn port(port: u32, seed: u8, children: Vec<TestItemPort>) -> TestItemPort {
    TestItemPort {
        port,
        item: [seed; 16],
        tail_count: 0,
        children,
    }
}

fn tree(root_children: Vec<TestItemPort>) -> TestChf {
    let mut test = TestChf::v8();
    test.ports = Some(port(0x1000, 0xa0, root_children));
    test
}

const fn bits(value: f32) -> u32 {
    value.to_bits()
}

#[test]
fn identical_files_have_no_changes() -> TestResult {
    assert_eq!(
        changes(&TestChf::v8(), &TestChf::v8())?,
        Vec::<String>::new()
    );
    Ok(())
}

#[test]
fn a_repeated_hash_pairs_equal_items_first_so_one_item_is_one_removal() -> TestResult {
    let mut old = TestChf::v8();
    old.materials[0].sub_materials[0].floats = vec![(0xf1, bits(1.0)), (0xf1, bits(2.0))];
    let mut new = old.clone();
    new.materials[0].sub_materials[0].floats = vec![(0xf1, bits(2.0))];

    assert_eq!(
        changes(&old, &new)?,
        [format!("{SUB}.floats.0x000000f1: removed (1)")]
    );
    Ok(())
}

#[test]
fn the_second_item_of_a_repeated_hash_is_shown_with_its_ordinal() -> TestResult {
    let mut old = TestChf::v8();
    old.materials[0].sub_materials[0].floats = vec![(0xf1, bits(1.0)), (0xf1, bits(2.0))];
    let mut new = old.clone();
    new.materials[0].sub_materials[0].floats = vec![(0xf1, bits(1.0)), (0xf1, bits(3.0))];

    assert_eq!(
        changes(&old, &new)?,
        [format!("{SUB}.floats.0x000000f1#1.value: 2 -> 3")]
    );
    Ok(())
}

#[test]
fn a_change_in_a_nested_child_port_is_named_by_its_path_of_ports() -> TestResult {
    let old = tree(vec![
        port(0x2000, 0xa1, vec![]),
        port(0x3000, 0xa2, vec![port(0x4000, 0xa3, vec![])]),
    ]);
    let new = tree(vec![
        port(0x2000, 0xa1, vec![]),
        port(0x3000, 0xa2, vec![port(0x4000, 0xb3, vec![])]),
    ]);

    assert_eq!(
        changes(&old, &new)?,
        [format!(
            "item_ports.0x00003000.0x00004000.item: {} -> {}",
            guid(0xa3),
            guid(0xb3)
        )]
    );
    Ok(())
}

#[test]
fn a_child_moved_to_another_port_is_a_removal_and_an_addition() -> TestResult {
    let old = tree(vec![
        port(0x2000, 0xa1, vec![]),
        port(0x3000, 0xa2, vec![port(0x4000, 0xa3, vec![])]),
    ]);
    let new = tree(vec![
        port(0x2000, 0xa1, vec![port(0x4000, 0xa3, vec![])]),
        port(0x3000, 0xa2, vec![]),
    ]);

    assert_eq!(
        changes(&old, &new)?,
        [
            format!(
                "item_ports.0x00002000.0x00004000: added (item {})",
                guid(0xa3)
            ),
            format!(
                "item_ports.0x00003000.0x00004000: removed (item {})",
                guid(0xa3)
            ),
        ]
    );
    Ok(())
}

#[test]
fn an_added_or_removed_subtree_is_one_change_with_its_child_count() -> TestResult {
    let small = tree(vec![port(0x2000, 0xa1, vec![])]);
    let big = tree(vec![
        port(0x2000, 0xa1, vec![]),
        port(0x5000, 0xa5, vec![port(0x6000, 0xa6, vec![])]),
    ]);

    let text = format!("item {}, 1 child port", guid(0xa5));
    assert_eq!(
        changes(&small, &big)?,
        [format!("item_ports.0x00005000: added ({text})")]
    );
    assert_eq!(
        changes(&big, &small)?,
        [format!("item_ports.0x00005000: removed ({text})")]
    );
    Ok(())
}

#[test]
fn textures_are_added_removed_and_changed_by_slot() -> TestResult {
    let mut old = TestChf::v8();
    old.materials[0].sub_materials[0].textures = vec![(1, [0x33; 16])];
    let mut new = old.clone();
    new.materials[0].sub_materials[0].textures = vec![(1, [0x34; 16]), (2, [0x35; 16])];

    assert_eq!(
        changes(&old, &new)?,
        [
            format!("{SUB}.textures.1.texture: {} -> {}", guid(0x33), guid(0x34)),
            format!("{SUB}.textures.2: added ({})", guid(0x35)),
        ]
    );
    assert_eq!(
        changes(&new, &old)?,
        [
            format!("{SUB}.textures.1.texture: {} -> {}", guid(0x34), guid(0x33)),
            format!("{SUB}.textures.2: removed ({})", guid(0x35)),
        ]
    );
    Ok(())
}

#[test]
fn a_texture_with_a_repeated_slot_is_not_dropped() -> TestResult {
    let mut old = TestChf::v8();
    old.materials[0].sub_materials[0].textures = vec![(1, [0x33; 16]), (1, [0x34; 16])];
    let mut new = old.clone();
    new.materials[0].sub_materials[0].textures = vec![(1, [0x34; 16])];
    let mut same_first = old.clone();
    same_first.materials[0].sub_materials[0].textures = vec![(1, [0x33; 16]), (1, [0x36; 16])];

    assert_eq!(
        changes(&old, &new)?,
        [format!("{SUB}.textures.1: removed ({})", guid(0x33))]
    );
    assert_eq!(
        changes(&old, &same_first)?,
        [format!(
            "{SUB}.textures.1#1.texture: {} -> {}",
            guid(0x34),
            guid(0x36)
        )]
    );
    Ok(())
}

#[test]
fn dna_on_one_side_only_is_present_or_absent() -> TestResult {
    let with = TestChf::v8();
    let mut without = TestChf::v8();
    without.dna_hashes = None;

    assert_eq!(changes(&with, &without)?, ["dna: removed (present)"]);
    assert_eq!(changes(&without, &with)?, ["dna: added (present)"]);
    Ok(())
}

#[test]
fn a_sub_material_on_one_side_only_is_listed_with_its_counts() -> TestResult {
    let old = TestChf::v8();
    let mut new = TestChf::v8();
    new.materials[0].sub_materials.pop();

    assert_eq!(
        changes(&old, &new)?,
        ["materials.0x0badcafe.sub_materials.0x00005ab2: removed (0 textures, 1 float, 0 colours)"]
    );
    Ok(())
}

#[test]
fn items_in_another_order_give_one_order_change() -> TestResult {
    let old = TestChf::v8();
    let mut new = TestChf::v8();
    new.materials[0].sub_materials.reverse();

    assert_eq!(
        changes(&old, &new)?,
        [
            "materials.0x0badcafe.sub_materials.order: 0x00005ab1, 0x00005ab2 -> 0x00005ab2, 0x00005ab1"
        ]
    );
    Ok(())
}

#[test]
fn merge_keeps_existing_names_and_keeps_the_kinds_apart() {
    let mut table = NameTable::builtin();
    table.add_sub_material("Shader_Mine");
    let mut other = NameTable::default();
    other.add_sub_material("shader_mine");
    other.add_sub_material("Shader_Other");
    other.add_param("Foo");

    table.merge(&other);

    assert_eq!(
        table.sub_material(lowered_hash("shader_mine")),
        Some("Shader_Mine")
    );
    assert_eq!(
        table.sub_material(lowered_hash("Shader_Other")),
        Some("Shader_Other")
    );
    assert_eq!(table.param(name_hash("Foo")), Some("Foo"));
    assert_eq!(table.sub_material(name_hash("Foo")), None);
    assert_eq!(table.port(name_hash("Foo")), None);
}

#[test]
fn merge_stops_at_the_limit_of_a_kind() {
    let mut full = NameTable::default();
    let mut index = 0_usize;
    while full.add_port(&format!("p{index}")) {
        index += 1;
    }
    assert!(index <= MAX_NAMES);
    let mut other = NameTable::default();
    other.add_port("extra_port");
    other.add_param("ExtraParam");

    full.merge(&other);

    assert_eq!(full.port(lowered_hash("extra_port")), None);
    assert_eq!(full.param(name_hash("ExtraParam")), Some("ExtraParam"));
}
