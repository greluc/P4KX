// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of an Entity reads its models through a model cache, so a rebuild reads none
//! from the Archive, and the cache's bound holds with the Interior models in it.

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, CachedModels, EntityInterior, EntityNode,
    InteriorOptions, InteriorWatch, LoadoutResolver, ModelCache, ModelFailure, ModelProvider,
    ProvidedModel,
};
use p4kx_extraction::model::MaterialStatus;
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{ENTITY_INTERIOR_CLASS, entity_interior_entries};

use crate::assembly_support::{TestResult, fixture};

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

fn build_with(
    archive: &Archive,
    (assembly, tree): (&Assembly, &EntityNode),
    models: &dyn ModelProvider,
) -> Result<EntityInterior, Box<dyn Error>> {
    Ok(EntityInterior::build_with_models(
        archive,
        assembly,
        tree,
        &InteriorOptions::default(),
        &InteriorWatch::none(),
        (None, Some(models)),
    )?)
}

#[test]
fn a_second_build_reads_no_interior_model_from_the_archive() -> TestResult {
    let fixture = fixture(entity_interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ENTITY_INTERIOR_CLASS)?;
    let tree = resolver.resolve(&record)?;
    let assembly = Assembly::build(
        &tree,
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default(),
    )?;
    let source = Counting {
        inner: ArchiveModels::new(&archive),
        calls: AtomicUsize::new(0),
    };
    let cache = ModelCache::default();
    let cached = CachedModels::new(&source, &cache);

    let first = build_with(&archive, (&assembly, &tree), &cached)?;
    let reads = source.calls.load(Ordering::SeqCst);
    let second = build_with(&archive, (&assembly, &tree), &cached)?;

    assert!(first.modelled_parts() > 0, "the Interior holds models");
    assert!(reads > 0, "the first build reads them");
    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        reads,
        "the second build reads none"
    );
    assert_eq!(second.parts(), first.parts());
    assert_eq!(second.modelled_parts(), first.modelled_parts());
    assert_eq!(second.issues().len(), first.issues().len());
    Ok(())
}

#[test]
fn the_bound_of_the_cache_holds_with_interior_models_in_it() -> TestResult {
    let fixture = fixture(entity_interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ENTITY_INTERIOR_CLASS)?;
    let tree = resolver.resolve(&record)?;
    let assembly = Assembly::build(
        &tree,
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default(),
    )?;
    let source = ArchiveModels::new(&archive);
    let unbounded = ModelCache::default();
    let full = build_with(
        &archive,
        (&assembly, &tree),
        &CachedModels::new(&source, &unbounded),
    )?;
    let limit = unbounded.bytes() / 2;
    assert!(limit > 0, "the Interior models take room");

    let bounded = ModelCache::new(limit);
    let built = build_with(
        &archive,
        (&assembly, &tree),
        &CachedModels::new(&source, &bounded),
    )?;

    assert!(bounded.bytes() <= limit, "{} > {limit}", bounded.bytes());
    assert!(bounded.len() < unbounded.len(), "some models were evicted");
    assert_eq!(built.modelled_parts(), full.modelled_parts());
    Ok(())
}
