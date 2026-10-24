// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_assembly::{ArchiveModels, Armour};
use p4kx_extraction::{
    CancelToken, ConversionError, GlbOptions, GlbSummary, GltfError, MaterialMode, WriteRefused,
    write_replacing,
};
use p4kx_settings::ModelMaterials;

use super::task::{Polled, Task};
use crate::assembly_export::{Observed, PartIssue, StoppableTextures, failure_of};
use crate::{EntryIndex, ModelExportFailure};

/// What the Export of a Character as GLB writes and where.
#[derive(Debug, Clone)]
pub(super) struct Request {
    pub(super) armour: Arc<Armour>,
    pub(super) name: String,
    pub(super) file: PathBuf,
    pub(super) materials: ModelMaterials,
    pub(super) texture_mip: u32,
    pub(super) vertex_colours: bool,
}

impl Request {
    fn options(&self) -> GlbOptions {
        let mode = match self.materials {
            ModelMaterials::Colours => MaterialMode::Colours,
            ModelMaterials::Textures => MaterialMode::Textures,
        };
        let options = GlbOptions::default()
            .with_lod(self.armour.lod())
            .with_materials(mode)
            .with_texture_mip(self.texture_mip);
        if self.vertex_colours {
            options
        } else {
            options.without_vertex_colours()
        }
    }
}

/// What a finished Export of a Character as GLB wrote and what it left out.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterExportSummary {
    file: PathBuf,
    name: String,
    parts: usize,
    lod: u32,
    written: GlbSummary,
    issues: Vec<PartIssue>,
}

impl CharacterExportSummary {
    /// Returns the GLB file that was written.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the name of the Character.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns how many parts the file holds.
    #[must_use]
    pub const fn parts(&self) -> usize {
        self.parts
    }

    /// Returns the level of detail that was exported.
    #[must_use]
    pub const fn lod(&self) -> u32 {
        self.lod
    }

    /// Returns what the writer counted.
    #[must_use]
    pub const fn written(&self) -> &GlbSummary {
        &self.written
    }

    /// Returns what could not be done in full, in the order of the pieces.
    #[must_use]
    pub fn issues(&self) -> &[PartIssue] {
        &self.issues
    }

    /// Returns how many pieces the report lists as left out of the file.
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.issues.iter().filter(|issue| issue.skipped).count()
    }
}

/// Where an Export of a Character as GLB stands.
#[derive(Debug, Clone, Copy)]
pub enum CharacterExportStatus<'a> {
    /// The export runs; the bytes written so far.
    Running(u64),
    /// The file was written.
    Finished(&'a CharacterExportSummary),
    /// The export was cancelled; no file was written.
    Cancelled,
    /// No file was written, for the reason given.
    Failed(&'a ModelExportFailure),
}

enum Ended {
    Written(Box<CharacterExportSummary>),
    Cancelled,
    Failed(ModelExportFailure),
}

#[derive(Debug)]
enum State {
    Running,
    Finished(Box<CharacterExportSummary>),
    Cancelled,
    Failed(ModelExportFailure),
}

/// An Export of a Character as GLB running on a background thread, or its outcome.
pub struct CharacterExport {
    id: u64,
    file: PathBuf,
    name: String,
    written: Arc<AtomicU64>,
    task: Option<Task<Ended>>,
    state: State,
}

impl std::fmt::Debug for CharacterExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharacterExport")
            .field("id", &self.id)
            .field("file", &self.file)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl CharacterExport {
    /// Returns the number that tells this export apart from every other one of the process.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the GLB file the export writes.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the name of the Character that is exported.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns where the export stands.
    #[must_use]
    pub fn status(&self) -> CharacterExportStatus<'_> {
        match &self.state {
            State::Running => CharacterExportStatus::Running(self.written.load(Ordering::Relaxed)),
            State::Finished(summary) => CharacterExportStatus::Finished(summary),
            State::Cancelled => CharacterExportStatus::Cancelled,
            State::Failed(failure) => CharacterExportStatus::Failed(failure),
        }
    }

    /// Returns whether the export still runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running)
    }

    /// Returns whether the export was asked to stop.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.task.as_ref().is_some_and(Task::is_cancelled)
    }

    /// Asks the export to stop; the file is not written then.
    pub fn cancel(&self) {
        if let Some(task) = &self.task {
            task.cancel();
        }
    }

    pub(super) fn poll(&mut self) -> bool {
        let Some(task) = &self.task else {
            return false;
        };
        let ended = match task.poll() {
            Polled::Pending => return false,
            Polled::Ready(ended) => Some(ended),
            Polled::Lost => None,
        };
        self.settle(ended);
        true
    }

    pub(super) fn stop(&mut self) {
        let Some(task) = &self.task else {
            return;
        };
        task.cancel();
        let ended = task.wait();
        self.settle(ended);
    }

    fn settle(&mut self, ended: Option<Ended>) {
        self.task = None;
        self.state = match ended {
            Some(Ended::Written(summary)) => State::Finished(summary),
            Some(Ended::Cancelled) => State::Cancelled,
            Some(Ended::Failed(failure)) => State::Failed(failure),
            None => State::Failed(ModelExportFailure::Stopped),
        };
    }
}

fn next_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// An export that did not start because `file` lies inside a Star Citizen installation.
pub(super) fn refused(file: PathBuf, name: String, refused: WriteRefused) -> CharacterExport {
    CharacterExport {
        id: next_id(),
        file,
        name,
        written: Arc::new(AtomicU64::new(0)),
        task: None,
        state: State::Failed(ModelExportFailure::Refused(refused)),
    }
}

pub(super) fn start(
    index: &Arc<EntryIndex>,
    request: Request,
    wake: Arc<dyn Fn() + Send + Sync>,
) -> CharacterExport {
    let id = next_id();
    let written = Arc::new(AtomicU64::new(0));
    let (file, name) = (request.file.clone(), request.name.clone());
    let thread_index = Arc::clone(index);
    let thread_written = Arc::clone(&written);
    let spawned = Task::spawn("character-export", wake, move |cancel| {
        run(&thread_index, &request, cancel, &thread_written)
    });
    let (task, state) = match spawned {
        Ok(task) => (Some(task), State::Running),
        Err(reason) => (None, State::Failed(ModelExportFailure::Failed(reason))),
    };
    CharacterExport {
        id,
        file,
        name,
        written,
        task,
        state,
    }
}

fn run(index: &EntryIndex, request: &Request, cancel: &CancelToken, written: &AtomicU64) -> Ended {
    if index.watch().check().is_err() {
        return Ended::Failed(ModelExportFailure::ArchiveChanged);
    }
    let models = ArchiveModels::new(index.archive());
    let archive_textures = models.textures();
    let textures = StoppableTextures {
        textures: &archive_textures,
        cancel,
    };
    let options = request.options();
    let outcome = write_replacing(&request.file, |writer| {
        let mut observed = Observed {
            inner: writer,
            cancel,
            written,
        };
        request
            .armour
            .write_glb(&options, &mut observed)
            .map_err(|error| match error {
                GltfError::Write(_) if cancel.is_cancelled() => ConversionError::Cancelled,
                GltfError::Write(source) => ConversionError::Write {
                    path: request.file.clone(),
                    source,
                },
                source => ConversionError::Glb {
                    file: request.name.clone(),
                    source,
                },
            })
    });
    match outcome {
        Ok(summary) => Ended::Written(Box::new(CharacterExportSummary {
            file: request.file.clone(),
            name: request.name.clone(),
            parts: request.armour.parts().len(),
            lod: request.armour.lod(),
            written: summary,
            issues: request.armour.issues().iter().map(PartIssue::of).collect(),
        })),
        Err(ConversionError::Cancelled) => Ended::Cancelled,
        Err(error) => Ended::Failed(failure_of(index, &error)),
    }
}
