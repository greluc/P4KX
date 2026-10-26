// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The helper bones of real static models sit where the nodes they drive sit.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::model::{LodSelection, Model, Node};
use p4kx_extraction::{Archive, ModelSource, PathIndex};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: usize = 400;
const TOLERANCE: f32 = 2.0e-3;

#[derive(Debug, Default)]
struct Tally {
    models: usize,
    bones: usize,
    apart: usize,
    named: usize,
    renamed: usize,
}

fn world_translation(nodes: &[Node], start: usize) -> Option<[f32; 3]> {
    let mut point = [0.0_f32; 3];
    let mut at = Some(start);
    for _ in 0..=nodes.len() {
        let Some(index) = at else {
            return Some(point);
        };
        let node = nodes.get(index)?;
        let source = point;
        for (value, row) in point.iter_mut().zip(&node.transform.rows) {
            let [x, y, z, shift] = *row;
            *value = x.mul_add(source[0], y.mul_add(source[1], z.mul_add(source[2], shift)));
        }
        at = node.parent;
    }
    None
}

#[expect(
    clippy::print_stderr,
    reason = "reports the bones that sit away from their node"
)]
fn check(path: &str, model: &Model, tally: &mut Tally) {
    let driving: Vec<_> = model
        .bones
        .iter()
        .filter_map(|bone| bone.node.map(|node| (bone, node)))
        .collect();
    if driving.is_empty() {
        return;
    }
    tally.models += 1;
    for (bone, node) in driving {
        let (Some(found), Some(world)) =
            (model.nodes.get(node), world_translation(&model.nodes, node))
        else {
            tally.apart += 1;
            eprintln!("{path}: bone {} names the missing node {node}", bone.name);
            continue;
        };
        tally.bones += 1;
        let bound = bone.world.translation;
        let distance = world
            .iter()
            .zip(&bound)
            .map(|(node, bone)| (node - bone).abs())
            .fold(0.0_f32, f32::max);
        let size = bound
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f32, f32::max);
        if distance > TOLERANCE * (1.0 + size) {
            tally.apart += 1;
            eprintln!(
                "{path}: bone {} at {bound:?}, node {} at {world:?}",
                bone.name, found.name
            );
        }
        if !bone.name.is_empty() && !found.name.is_empty() {
            tally.named += 1;
            tally.renamed += usize::from(!bone.name.eq_ignore_ascii_case(&found.name));
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stderr,
    reason = "reports what the sample of the real Archive showed"
)]
fn the_helper_bones_of_real_models_sit_where_the_nodes_they_drive_sit() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let paths = PathIndex::new(&archive);
    let all: Vec<&str> = archive
        .entries()
        .iter()
        .map(p4kx_extraction::Entry::path)
        .filter(|path| {
            Path::new(path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("cga"))
                && path.to_ascii_lowercase().contains("objects")
        })
        .collect();
    let stride = (all.len() / SAMPLE).max(1);
    let mut tally = Tally::default();
    for path in all.iter().step_by(stride) {
        if let Ok(source) = ModelSource::from_index_with(&paths, path, LodSelection::Only(0)) {
            check(path, source.model(), &mut tally);
        }
    }
    eprintln!("{tally:?}");
    assert!(tally.models > 0, "the sample has models with helper bones");
    assert_eq!(
        tally.apart, 0,
        "every helper bone sits at the node it drives"
    );
    assert_eq!(tally.renamed, 0, "and carries its name");
    Ok(())
}
