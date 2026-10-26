// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The model caches with the models of a real ship's hull and Interior.

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, CachedModels,
    DEFAULT_MODEL_CACHE_BYTES, EntityInterior, EntityNode, InteriorOptions, InteriorWatch,
    LoadoutResolver, ModelCache, ModelFailure, ModelProvider, ProvidedModel,
};
use p4kx_extraction::model::MaterialStatus;
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const INTERIOR_CACHE_BYTES: usize = 512 * 1024 * 1024;

struct Counting<'a> {
    inner: &'a dyn ModelProvider,
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

fn build(
    archive: &Archive,
    tree: &EntityNode,
    (outer, inner): (&dyn ModelProvider, &dyn ModelProvider),
) -> Result<(Assembly, EntityInterior), Box<dyn Error>> {
    let assembly = Assembly::build(tree, outer, &AssemblyOptions::default())?;
    let interior = EntityInterior::build_with_models(
        archive,
        &assembly,
        tree,
        &InteriorOptions::default(),
        &InteriorWatch::none(),
        (None, Some(inner)),
    )?;
    Ok((assembly, interior))
}

fn repeated_reads(archive: &Archive, tree: &EntityNode) -> Result<(usize, usize), Box<dyn Error>> {
    let source = ArchiveModels::new(archive);
    let (cache, interior_cache) = (ModelCache::default(), ModelCache::new(INTERIOR_CACHE_BYTES));
    let reads = Counting {
        inner: &source,
        calls: AtomicUsize::new(0),
    };
    let (cached, interior_cached) = (
        CachedModels::new(&reads, &cache),
        CachedModels::new(&reads, &interior_cache),
    );
    let (outer, inner) = (
        Counting {
            inner: &cached,
            calls: AtomicUsize::new(0),
        },
        Counting {
            inner: &interior_cached,
            calls: AtomicUsize::new(0),
        },
    );
    build(archive, tree, (&outer, &inner))?;
    let asked = || outer.calls.load(Ordering::SeqCst) + inner.calls.load(Ordering::SeqCst);
    let (first_requests, first_reads) = (asked(), reads.calls.load(Ordering::SeqCst));
    build(archive, tree, (&outer, &inner))?;
    Ok((
        asked() - first_requests,
        reads.calls.load(Ordering::SeqCst) - first_reads,
    ))
}

fn check(class_name: &str) -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(class_name)?;
    let tree = resolver.resolve(&record)?;

    let source = ArchiveModels::new(&archive);
    let demand = ModelCache::new(usize::MAX);
    let all = CachedModels::new(&source, &demand);
    build(&archive, &tree, (&all, &all))?;
    assert!(
        demand.bytes() > DEFAULT_MODEL_CACHE_BYTES,
        "{class_name}: the hull and the Interior take {} bytes, more than one default cache",
        demand.bytes()
    );

    let (requests, reads) = repeated_reads(&archive, &tree)?;
    assert!(
        requests > 0,
        "{class_name}: the second build asks for models"
    );
    assert!(
        reads * 20 < requests,
        "{class_name}: {reads} of {requests} requests of the second build read the Archive"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_caches_hold_the_models_of_a_carrack_and_its_interior() -> TestResult {
    check("ANVL_Carrack")
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_caches_hold_the_models_of_an_idris_and_its_interior() -> TestResult {
    check("AEGS_Idris_P")
}
