// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A part under a parent without a model, and the vertex budget while the models are read.

use crate::assembly_support;

use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};

use assembly_support::{TestResult, assemble, entries, fixture};
use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver, ModelFailure, ModelProvider,
    ProvidedModel,
};
use p4kx_extraction::model::MaterialStatus;
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestModel,
    entity_schema,
};

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

struct EveryPathIsTheCube<'a> {
    inner: ArchiveModels<'a>,
    calls: AtomicUsize,
}

impl ModelProvider for EveryPathIsTheCube<'_> {
    fn model(&self, _path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.model("Objects/Test/cube.cgf", lod)
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.inner.library(entry, name)
    }
}

const FILES: usize = 40;

fn many_files() -> Vec<TestEntry> {
    let leaves = (0..FILES).map(|n| {
        TestEntity::new(
            0x100 + n as u128,
            format!("Leaf_{n:02}"),
            format!("entities/leaf_{n:02}.xml"),
        )
        .geometry(format!("Objects/Test/leaf_{n:02}.cgf"), None)
    });
    let entries: Vec<TestLoadoutEntry> = (0..FILES)
        .map(|n| TestLoadoutEntry::class(format!("port_{n:02}"), format!("Leaf_{n:02}")))
        .collect();
    let root =
        TestEntity::new(0x1, "Root", "entities/root.xml").loadout(TestLoadout::Manual(entries));
    let database = std::iter::once(root)
        .chain(leaves)
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        });
    let model = TestModel::cube();
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database.build()),
        TestEntry::stored("Data\\Objects\\Test\\cube.cgf", model.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\cube.cgfm", model.cgfm()),
    ]
}

fn modelled_paths(
    archive: &Archive,
    budget: usize,
) -> Result<(Vec<String>, usize), Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find("Root")?;
    let tree = resolver.resolve(&record)?;
    let provider = EveryPathIsTheCube {
        inner: ArchiveModels::new(archive),
        calls: AtomicUsize::new(0),
    };
    let options = AssemblyOptions::default().with_limits(100, budget);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build()?;
    let assembly = pool.install(|| Assembly::build(&tree, &provider, &options))?;
    let paths = assembly
        .parts()
        .iter()
        .filter(|part| part.model().is_some())
        .map(|part| part.path().to_owned())
        .collect();
    Ok((paths, provider.calls.load(Ordering::SeqCst)))
}

#[test]
fn the_budget_holds_over_several_waves_and_the_same_parts_get_models_every_time() -> TestResult {
    let fixture = fixture(many_files())?;
    let archive = Archive::open(&fixture.path)?;

    let first = modelled_paths(&archive, 24 * 10)?;
    let second = modelled_paths(&archive, 24 * 10)?;

    let expected: Vec<String> = (0..10).map(|n| format!("port_{n:02}")).collect();
    assert_eq!(first.0, expected, "the first ten parts fit the budget");
    assert_eq!(second, first, "the same Assembly and the same reads");
    assert!(
        first.1 < FILES,
        "{} of {FILES} files were read; the rest follow the budget",
        first.1
    );
    assert!(
        first.1 <= 10 + 8,
        "one wave of eight beyond the budget at most"
    );
    Ok(())
}
