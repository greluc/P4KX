// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Where the nodes below a skinned mesh's node are written.

use std::error::Error;

use p4kx_testkit::{TestModel, TestNode, TestSkeleton};

use crate::skin::{glb, global};

type TestResult = Result<(), Box<dyn Error>>;

fn node(name: &str, parent: Option<u16>, x: f32, kind: u16) -> TestNode {
    TestNode {
        name: name.to_owned(),
        parent,
        local: [
            [1.0, 0.0, 0.0, x],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
        kind,
    }
}

#[test]
fn a_node_below_the_skinned_mesh_keeps_the_full_transform_chain() -> TestResult {
    let source = TestModel::cube()
        .with_nodes(vec![
            node("cube", None, 5.0, 2),
            node("cube_mesh", Some(0), 3.0, 0),
            node("tag", Some(1), 0.0, 2),
        ])
        .with_skeleton(&TestSkeleton::two_bones())
        .with_bone_map(false, vec![vec![(1, 255)]; 24]);
    let (glb, _) = glb(&source)?;

    let find = |name: &str| {
        glb.document
            .nodes()
            .find(|node| node.name() == Some(name))
            .ok_or_else(|| format!("no node {name}"))
    };
    let (tag, cube) = (find("tag")?, find("cube")?);
    assert!(
        cube.children().any(|child| child.index() == tag.index()),
        "the tag hangs below the nearest node that is not skinned"
    );
    let [_, _, _, [x, _, _, _]] = global(&glb, tag.index());
    assert!(
        (x - 8.0).abs() < 1e-5,
        "the tag keeps the moves above and of the skinned node: {x}"
    );
    Ok(())
}

#[test]
fn a_node_below_a_skinned_root_takes_over_its_transform() -> TestResult {
    let source = TestModel::cube()
        .with_nodes(vec![
            node("other", None, 0.0, 2),
            node("cube_mesh", None, 3.0, 0),
            node("tag", Some(1), 1.0, 2),
        ])
        .with_skeleton(&TestSkeleton::two_bones())
        .with_bone_map(false, vec![vec![(1, 255)]; 24]);
    let (glb, _) = glb(&source)?;

    let tag = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("tag"))
        .ok_or("the tag node")?;
    let [_, _, _, [x, _, _, _]] = global(&glb, tag.index());
    assert!((x - 4.0).abs() < 1e-5, "the tag stands at 3 + 1: {x}");
    Ok(())
}
