// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

pub(super) mod stop;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyError, AssemblyOptions, ForgePalettes, LoadoutChanges,
};
use p4kx_extraction::{
    CancelToken, ConversionError, GlbOptions, GlbSummary, GltfError, Guid, MaterialMode,
    archive_error_in, write_replacing,
};
use p4kx_settings::ModelMaterials;
use stop::{Observed, Stoppable, StoppableTextures};

use super::AssemblyExportSummary;
use crate::entities::{ResolveFailure, resolve};
use crate::forge_cache::ReadFailure;
use crate::model::describe;
use crate::{EntryIndex, ModelExportFailure};

/// What an Export of an Entity as GLB writes and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub(crate) guid: Guid,
    pub(crate) name: String,
    pub(crate) changes: LoadoutChanges,
    pub(crate) file: PathBuf,
    pub(crate) lod: u32,
    pub(crate) materials: ModelMaterials,
    pub(crate) texture_mip: u32,
    pub(crate) vertex_colours: bool,
}

impl Request {
    fn options(&self) -> GlbOptions {
        let mode = match self.materials {
            ModelMaterials::Colours => MaterialMode::Colours,
            ModelMaterials::Textures => MaterialMode::Textures,
        };
        let options = GlbOptions::default()
            .with_lod(self.lod)
            .with_materials(mode)
            .with_texture_mip(self.texture_mip);
        if self.vertex_colours {
            options
        } else {
            options.without_vertex_colours()
        }
    }
}

/// The step a running Export of an Entity as GLB is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyExportStage {
    /// The Loadout of the Entity is resolved.
    Resolving,
    /// The models of the parts are read from the Archive.
    Reading,
    /// The GLB file is written.
    Writing,
}

/// How far a running Export of an Entity as GLB has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssemblyExportProgress {
    /// The step it is at.
    pub stage: AssemblyExportStage,
    /// The bytes of the file written so far.
    pub done: u64,
}

#[derive(Debug, Default)]
struct Shared {
    stage: AtomicU8,
    done: AtomicU64,
}

impl Shared {
    fn progress(&self) -> AssemblyExportProgress {
        AssemblyExportProgress {
            stage: match self.stage.load(Ordering::Relaxed) {
                0 => AssemblyExportStage::Resolving,
                1 => AssemblyExportStage::Reading,
                _ => AssemblyExportStage::Writing,
            },
            done: self.done.load(Ordering::Relaxed),
        }
    }

    fn enter(&self, stage: AssemblyExportStage) {
        let code = match stage {
            AssemblyExportStage::Resolving => 0,
            AssemblyExportStage::Reading => 1,
            AssemblyExportStage::Writing => 2,
        };
        self.stage.store(code, Ordering::Relaxed);
    }
}

/// Where an Export of an Entity as GLB stands.
#[derive(Debug, Clone, Copy)]
pub enum AssemblyExportStatus<'a> {
    /// The export runs; the progress so far.
    Running(AssemblyExportProgress),
    /// The file was written.
    Finished(&'a AssemblyExportSummary),
    /// The export was cancelled; no file was written.
    Cancelled,
    /// No file was written, for the reason given.
    Failed(&'a ModelExportFailure),
}

enum Ended {
    Written(Box<AssemblyExportSummary>),
    Cancelled,
    Failed(ModelExportFailure),
}

#[derive(Debug)]
enum State {
    Running(Receiver<Ended>),
    Finished(Box<AssemblyExportSummary>),
    Cancelled,
    Failed(ModelExportFailure),
}

impl std::fmt::Debug for Ended {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ended").finish_non_exhaustive()
    }
}

/// An Export of an Entity as GLB running on a background thread, or its outcome.
#[derive(Debug)]
pub struct AssemblyExport {
    id: u64,
    file: PathBuf,
    name: String,
    cancel: CancelToken,
    shared: Arc<Shared>,
    state: State,
}

impl AssemblyExport {
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

    /// Returns the class name of the Entity that is exported.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns where the export stands.
    #[must_use]
    pub fn status(&self) -> AssemblyExportStatus<'_> {
        match &self.state {
            State::Running(_) => AssemblyExportStatus::Running(self.shared.progress()),
            State::Finished(summary) => AssemblyExportStatus::Finished(summary),
            State::Cancelled => AssemblyExportStatus::Cancelled,
            State::Failed(failure) => AssemblyExportStatus::Failed(failure),
        }
    }

    /// Returns whether the export still runs.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Returns whether the export was asked to stop.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Asks the export to stop; the file is not written then.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Takes the outcome once the export ended; returns whether it ended.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Running(result) = &self.state else {
            return false;
        };
        self.state = match result.try_recv() {
            Ok(ended) => settle(Some(ended)),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => settle(None),
        };
        true
    }

    /// Cancels the export and waits until it ended, the partial file removed.
    pub(crate) fn stop(&mut self) {
        self.cancel.cancel();
        if let State::Running(result) = &self.state {
            self.state = settle(result.recv().ok());
        }
    }
}

fn settle(ended: Option<Ended>) -> State {
    match ended {
        Some(Ended::Written(summary)) => State::Finished(summary),
        Some(Ended::Cancelled) => State::Cancelled,
        Some(Ended::Failed(failure)) => State::Failed(failure),
        None => State::Failed(ModelExportFailure::Stopped),
    }
}

/// Starts exporting the Entity of `request` from `index` on a background thread and calls `wake`
/// from it once the export ended.
pub(crate) fn start(
    index: &Arc<EntryIndex>,
    request: Request,
    wake: impl Fn() + Send + 'static,
) -> AssemblyExport {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let cancel = CancelToken::new();
    let shared = Arc::new(Shared::default());
    let (file, name) = (request.file.clone(), request.name.clone());
    let (sender, result) = mpsc::channel();
    let thread_index = Arc::clone(index);
    let thread_cancel = cancel.clone();
    let thread_shared = Arc::clone(&shared);
    let spawned = thread::Builder::new()
        .name("assembly-export".to_owned())
        .spawn(move || {
            let ended = run(&thread_index, &request, &thread_cancel, &thread_shared);
            if sender.send(ended).is_ok() {
                wake();
            }
        });
    let state = match spawned {
        Ok(_handle) => State::Running(result),
        Err(error) => State::Failed(ModelExportFailure::Failed(describe(&error))),
    };
    AssemblyExport {
        id,
        file,
        name,
        cancel,
        shared,
        state,
    }
}

#[cfg(test)]
mod tests;

fn run(index: &EntryIndex, request: &Request, cancel: &CancelToken, shared: &Shared) -> Ended {
    let tree = match resolve(index, request.guid, &request.changes) {
        Ok(tree) => tree,
        Err(ResolveFailure::ArchiveChanged) => {
            return Ended::Failed(ModelExportFailure::ArchiveChanged);
        }
        Err(ResolveFailure::Failed(reason)) => {
            return Ended::Failed(ModelExportFailure::Failed(reason));
        }
    };
    if cancel.is_cancelled() {
        return Ended::Cancelled;
    }
    shared.enter(AssemblyExportStage::Reading);
    let source = match index.forge().read(index, || {}) {
        Ok(source) => source,
        Err(ReadFailure::ArchiveChanged(_)) => {
            return Ended::Failed(ModelExportFailure::ArchiveChanged);
        }
        Err(ReadFailure::Unreadable(reason)) => {
            return Ended::Failed(ModelExportFailure::Failed(reason));
        }
    };
    let forge = match source.entry.open() {
        Ok(forge) => forge,
        Err(error) => return Ended::Failed(ModelExportFailure::Failed(describe(&error))),
    };
    let palettes = ForgePalettes::new(&forge);
    let archive_models = ArchiveModels::new(index.archive());
    let models = Stoppable {
        models: &archive_models,
        cancel,
    };
    let options = AssemblyOptions::default().with_lod(request.lod);
    let assembly = match Assembly::build_with_palettes(&tree, &models, &palettes, &options) {
        Ok(assembly) => assembly,
        Err(_) if cancel.is_cancelled() => return Ended::Cancelled,
        Err(AssemblyError::Provider(error)) => return failed(index, &*error),
        Err(other) => return failed(index, &other),
    };
    if cancel.is_cancelled() {
        return Ended::Cancelled;
    }
    if index.watch().check().is_err() {
        return Ended::Failed(ModelExportFailure::ArchiveChanged);
    }
    shared.enter(AssemblyExportStage::Writing);
    match write(&assembly, &archive_models, request, (cancel, shared)) {
        Ok(written) => Ended::Written(Box::new(AssemblyExportSummary::new(
            (request.file.clone(), request.name.clone()),
            &assembly,
            written,
            &request.changes,
        ))),
        Err(ConversionError::Cancelled) => Ended::Cancelled,
        Err(error) => failed(index, &error),
    }
}

fn write(
    assembly: &Assembly,
    models: &ArchiveModels<'_>,
    request: &Request,
    (cancel, shared): (&CancelToken, &Shared),
) -> Result<GlbSummary, ConversionError> {
    let glb = request.options();
    let archive_textures = models.textures();
    let textures = StoppableTextures {
        textures: &archive_textures,
        cancel,
    };
    write_replacing(&request.file, |writer| {
        let mut observed = Observed {
            inner: writer,
            cancel,
            shared,
        };
        assembly
            .write_glb_with(&glb, &textures, &mut observed)
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
    })
}

fn failed(index: &EntryIndex, error: &(dyn std::error::Error + 'static)) -> Ended {
    Ended::Failed(failure_of(index, error))
}

/// Returns why reading or writing an Assembly failed with `error`.
pub(crate) fn failure_of(
    index: &EntryIndex,
    error: &(dyn std::error::Error + 'static),
) -> ModelExportFailure {
    if index.watch().check().is_err() {
        return ModelExportFailure::ArchiveChanged;
    }
    if let Some(archive) = archive_error_in(error) {
        if archive.is_key_required() {
            return ModelExportFailure::KeyRequired;
        }
        if archive.is_wrong_key() {
            return ModelExportFailure::KeyRejected;
        }
    }
    if let Some(ConversionError::Installation(refused)) = error.downcast_ref::<ConversionError>() {
        return ModelExportFailure::Refused(refused.clone());
    }
    ModelExportFailure::Failed(describe(error))
}
