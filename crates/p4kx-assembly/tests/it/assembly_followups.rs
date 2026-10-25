// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A part under a parent without a model, and the vertex budget while the models are read.

use crate::assembly_support;

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use assembly_support::{TestResult, assemble, entries, fixture};
use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, EntityNode, LoadoutResolver, ModelFailure,
    ModelProvider, ProvidedModel,
};
use p4kx_extraction::model::MaterialStatus;
use p4kx_extraction::{Archive, DataForgeEntry};

#[test]
fn a_child_of_a_part_without_model_reports_no_helper() -> TestResult {
    let kept: Vec<_> = entries()
        .into_iter()
        .filter(|entry| !format!("{entry:?}").contains("mount.cgf"))
        .collect();
    let fixture = fixture(kept)?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, "TEST_Ship", 0)?;

    let codes: Vec<(&str, &str)> = assembly
        .issues()
        .iter()
        .map(|issue| (issue.path(), issue.kind().code()))
        .collect();
    assert!(
        codes.contains(&("hardpoint_weapon_nose", "geometry_missing")),
        "{codes:?}"
    );
    assert!(
        !codes
            .iter()
            .any(|(path, code)| path.ends_with("hardpoint_class_2") && *code == "helper_not_found"),
        "the gun's parent has no model, which the mount's own issue already says: {codes:?}"
    );
    Ok(())
}

struct Counting<'a> {
    inner: ArchiveModels<'a>,
    calls: AtomicUsize,
}

impl ModelProvider for Counting<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.model(path, lod)
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.inner.library(entry, name)
    }
}

fn tree(archive: &Archive) -> Result<EntityNode, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find("TEST_Ship")?;
    Ok(resolver.resolve(&record)?)
}

#[test]
fn no_model_is_read_once_the_budget_is_spent_within_a_wave() -> TestResult {
    let fixture = fixture(entries())?;
    let archive = Archive::open(&fixture.path)?;
    let tree = tree(&archive)?;
    let provider = Counting {
        inner: ArchiveModels::new(&archive),
        calls: AtomicUsize::new(0),
    };
    let options = AssemblyOptions::default().with_limits(100, 1);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build()?;

    let assembly = pool.install(|| Assembly::build(&tree, &provider, &options))?;

    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        1,
        "the ship alone holds the budget; the other files of the wave are not read"
    );
    assert_eq!(
        assembly.modelled_parts(),
        0,
        "24 vertices exceed the budget"
    );
    Ok(())
}
