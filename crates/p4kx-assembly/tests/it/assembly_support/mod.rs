// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The synthetic ship of `assembly_entries` in an Archive on disk, and a way to assemble it.

use std::error::Error;
use std::path::PathBuf;

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

pub(crate) use p4kx_testkit::assembly_entries as entries;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) struct Fixture {
    _dir: tempfile::TempDir,
    pub(crate) path: PathBuf,
}

pub(crate) fn fixture(entries: Vec<TestEntry>) -> Result<Fixture, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok(Fixture { _dir: dir, path })
}

pub(crate) fn assemble(
    archive: &Archive,
    entity: &str,
    lod: u32,
) -> Result<Assembly, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(entity)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(archive);
    Ok(Assembly::build(
        &tree,
        &models,
        &AssemblyOptions::default().with_lod(lod),
    )?)
}
