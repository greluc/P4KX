// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Census of the object containers of every vehicle Entity: where the bone of the hull agrees
//! with the Record's offset.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, EntityNode, LoadoutResolver, ModelProvider, ObjectContainerRef,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_model::{Model, Transform};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const TOLERANCE: f32 = 0.05;

#[derive(Default, Debug)]
struct Census {
    entities: usize,
    containers: usize,
    no_bone_named: usize,
    no_model: usize,
    bone_missing: usize,
    offset_identity: usize,
    agree: usize,
    disagree: usize,
    examples: Vec<String>,
}

pub(super) fn bone_translation(model: &Model, name: &str) -> Option<[f32; 3]> {
    let mut at = model
        .nodes
        .iter()
        .position(|node| node.name.eq_ignore_ascii_case(name))?;
    let mut chain = Transform::IDENTITY;
    for _ in 0..=model.nodes.len() {
        let node = model.nodes.get(at)?;
        chain = node.transform.then(&chain);
        match node.parent {
            Some(parent) => at = parent,
            None => return Some(chain.translation()),
        }
    }
    None
}

pub(super) fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= TOLERANCE)
}

fn count(census: &mut Census, tree: &EntityNode, hull: Option<&Model>, r: &ObjectContainerRef) {
    census.containers += 1;
    if r.bone().is_empty() {
        census.no_bone_named += 1;
        return;
    }
    let Some(hull) = hull else {
        census.no_model += 1;
        return;
    };
    let Some(bone) = bone_translation(hull, r.bone()) else {
        census.bone_missing += 1;
        return;
    };
    if near(r.position(), [0.0; 3]) {
        census.offset_identity += 1;
    } else if near(r.position(), bone) {
        census.agree += 1;
    } else {
        census.disagree += 1;
        if census.examples.len() < 12 {
            census.examples.push(format!(
                "{} {} bone {:?} offset {:?}",
                tree.class_name(),
                r.bone(),
                bone,
                r.position()
            ));
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the census is read from the output")]
fn census_of_bones_and_offsets() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let models = ArchiveModels::new(&archive);
    let records: Vec<_> = forge
        .records()
        .filter(|record| {
            record
                .type_name()
                .is_ok_and(|name| name == "EntityClassDefinition")
        })
        .collect();
    let mut census = Census::default();
    for record in &records {
        let Ok(tree) = resolver.resolve(record) else {
            continue;
        };
        if tree.object_containers().is_empty() {
            continue;
        }
        census.entities += 1;
        let hull = tree
            .geometry()
            .and_then(|geometry| models.model(geometry.path(), 0).ok());
        for r in tree.object_containers() {
            count(&mut census, &tree, hull.as_ref().map(|h| &h.model), r);
        }
    }
    println!("{census:#?}");
    assert!(census.containers > 100, "{}", census.containers);
    Ok(())
}
