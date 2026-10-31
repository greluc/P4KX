// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Shaping a head reads the `RigLogic` chunk the provider handed along with the skin, not a
//! second copy of the skin file.

use std::sync::Mutex;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, Expressions, ModelFailure, ModelProvider,
    Piece, ProvidedModel,
};
use p4kx_extraction::Archive;
use p4kx_extraction::model::{MaterialStatus, Model};
use p4kx_testkit::{HEAD_VERTICES, MALE_RIG_HASH, head_entries, head_rig};

use crate::assembly_support::{TestResult, fixture};
use crate::head_build::{dna_of_head_two_in_region_three, head_path};

/// Delegates to `inner` and records the paths of the files it is asked for.
struct Recording<'a> {
    inner: ArchiveModels<'a>,
    files: Mutex<Vec<String>>,
}

impl ModelProvider for Recording<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        self.inner.model(path, lod)
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.inner.library(entry, name)
    }

    fn file(&self, path: &str, limit: u64) -> Result<Vec<u8>, ModelFailure> {
        self.files
            .lock()
            .map_err(|_| ModelFailure::Missing)?
            .push(path.to_owned());
        self.inner.file(path, limit)
    }
}

#[test]
fn the_provider_hands_the_rig_logic_chunk_along_with_the_skin() -> TestResult {
    let fixture = fixture(head_entries(Some(
        head_rig(HEAD_VERTICES, MALE_RIG_HASH).build(),
    )))?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);

    let provided = models.model(&head_path(), 0)?;

    let chunk: &Model = &provided.model;
    assert!(
        chunk
            .rig_logic
            .as_deref()
            .is_some_and(|bytes| !bytes.is_empty())
    );
    Ok(())
}

#[test]
fn shaping_reads_no_second_copy_of_the_skin() -> TestResult {
    let fixture = fixture(head_entries(Some(
        head_rig(HEAD_VERTICES, MALE_RIG_HASH).build(),
    )))?;
    let archive = Archive::open(&fixture.path)?;
    let models = Recording {
        inner: ArchiveModels::new(&archive),
        files: Mutex::new(Vec::new()),
    };
    let piece = Piece::file("head", head_path(), None)
        .shaped(dna_of_head_two_in_region_three())
        .with_expressions(Expressions::All);

    let armour = Armour::build(
        BodyVariant::Male,
        &[piece],
        &models,
        &AssemblyOptions::default(),
    )?;

    let head = armour
        .parts()
        .iter()
        .find(|part| part.name() == "head")
        .ok_or("the head part")?;
    assert_eq!(head.morphs().len(), 2);
    let files = models
        .files
        .into_inner()
        .map_err(|_| "the recorded files")?;
    assert!(
        !files
            .iter()
            .any(|path| path.eq_ignore_ascii_case(&head_path())),
        "{files:?}"
    );
    Ok(())
}
