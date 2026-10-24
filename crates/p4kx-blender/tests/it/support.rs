// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The synthetic ships of the testkit in an Archive on disk, and a way to assemble them.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{ArchiveBuilder, TestEntry};

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

pub(crate) fn assemble(archive: &Archive, entity: &str) -> Result<Assembly, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(entity)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(archive);
    let palettes = ForgePalettes::new(&forge);
    Ok(Assembly::build_with_palettes(
        &tree,
        &models,
        &palettes,
        &AssemblyOptions::default(),
    )?)
}

pub(crate) fn glb_options(mip: u32) -> GlbOptions {
    GlbOptions::default().with_texture_mip(mip)
}

pub(crate) const fn never() -> bool {
    false
}

pub(crate) fn names_in(folder: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names: Vec<String> = std::fs::read_dir(folder)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?;
    names.sort();
    Ok(names)
}
