// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior reads its models through a model cache, so a rebuild reads none from the
//! Archive, the cache's bound holds with the Interior models in it, and a model that only a
//! container holds is never cached.

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, CachedModels, EntityInterior, EntityNode, Interior,
    InteriorOptions, InteriorWatch, LoadoutResolver, ModelCache, ModelFailure, ModelProvider,
    ProvidedModel,
};
use p4kx_extraction::model::{MaterialStatus, Model};
use p4kx_extraction::{Archive, DataForgeEntry, select_entries};
use p4kx_testkit::{
    ENTITY_INTERIOR_CLASS, SOC_IDENTITY, TestContainer, TestEntry, TestModel, TestObjectContainer,
    TestSoc, entity_interior_entries,
};

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

fn with_entity(run: impl FnOnce(&Archive, (&Assembly, &EntityNode)) -> TestResult) -> TestResult {
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
    run(&archive, (&assembly, &tree))
}

#[test]
fn a_second_build_reads_no_interior_model_from_the_archive() -> TestResult {
    with_entity(|archive, scene| {
        let source = Counting {
            inner: ArchiveModels::new(archive),
            calls: AtomicUsize::new(0),
        };
        let cache = ModelCache::default();
        let cached = CachedModels::new(&source, &cache);

        let first = build_with(archive, scene, &cached)?;
        let reads = source.calls.load(Ordering::SeqCst);
        let second = build_with(archive, scene, &cached)?;

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
    })
}

#[test]
fn the_bound_of_the_cache_holds_with_interior_models_in_it() -> TestResult {
    with_entity(|archive, scene| {
        let source = ArchiveModels::new(archive);
        let unbounded = ModelCache::default();
        let full = build_with(archive, scene, &CachedModels::new(&source, &unbounded))?;
        let limit = unbounded.bytes() / 2;
        assert!(limit > 0, "the Interior models take room");

        let bounded = ModelCache::new(limit);
        let built = build_with(archive, scene, &CachedModels::new(&source, &bounded))?;

        assert!(bounded.bytes() <= limit, "{} > {limit}", bounded.bytes());
        assert!(bounded.len() < unbounded.len(), "some models were evicted");
        assert_eq!(built.modelled_parts(), full.modelled_parts());
        Ok(())
    })
}

const LOCAL: &str = "Data\\ObjectContainers\\local.socpak";

fn extent(model: &Model) -> f32 {
    model
        .lods
        .iter()
        .flat_map(|lod| &lod.meshes)
        .flat_map(|mesh| &mesh.positions)
        .flatten()
        .fold(0.0, |most, value| f32::max(most, value.abs()))
}

fn model_entries(path: &str, model: &TestModel) -> Vec<TestEntry> {
    vec![
        TestEntry::stored(format!("{path}.cgf"), model.cgf()),
        TestEntry::stored(format!("{path}.cgfm"), model.cgfm()),
    ]
}

#[test]
fn a_model_the_container_holds_beats_the_archives_and_is_not_cached() -> TestResult {
    let soc = TestSoc::new()
        .model("objects/test/cube.cgf")
        .model("objects/test/other.cgf")
        .object(0, SOC_IDENTITY)
        .object(1, SOC_IDENTITY);
    let mut local = TestContainer::new()
        .entry(TestEntry::stored(
            "local.xml",
            TestObjectContainer::new("local").stream("local.soc").xml(),
        ))
        .entry(TestEntry::deflated("local.soc", soc.bytes()));
    for entry in model_entries("Objects\\Test\\cube", &TestModel::cube().scaled(4.0)) {
        local = local.entry(entry);
    }
    let mut entries = model_entries("Data\\Objects\\Test\\cube", &TestModel::cube());
    entries.extend(model_entries(
        "Data\\Objects\\Test\\other",
        &TestModel::cube(),
    ));
    entries.push(TestEntry::stored(LOCAL, local.build()));
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[LOCAL])?;
    let root = archive.open_container(found.first().ok_or("no container")?)?;
    let source = ArchiveModels::new(&archive);
    let cache = ModelCache::default();

    let interior = Interior::build_with_models(
        &[&archive, &root],
        LOCAL,
        &InteriorOptions::default(),
        &InteriorWatch::none(),
        (None, Some(&CachedModels::new(&source, &cache))),
    )?;

    let extents: Vec<(Option<&str>, f32)> = interior
        .parts()
        .iter()
        .filter_map(|part| Some((part.model_path(), extent(part.model()?))))
        .collect();
    let of = |name: &str| {
        extents
            .iter()
            .find(|(path, _)| path.is_some_and(|path| path.ends_with(name)))
            .map(|(_, extent)| *extent)
    };
    let archive_cube = extent(&source.model("objects/test/cube.cgf", 0)?.model);
    let local_cube = of("cube.cgf").ok_or("no cube part")?;
    assert!(
        local_cube > archive_cube * 3.0,
        "the container's cube: {local_cube} against the Archive's {archive_cube}"
    );
    assert_eq!(of("other.cgf"), Some(archive_cube));
    assert_eq!(cache.len(), 1, "only the Archive's other model is cached");
    Ok(())
}
