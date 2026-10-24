// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyError, AssemblyOptions, EntityNode, ForgePalettes,
    ModelFailure, ModelProvider, ProvidedModel,
};
use p4kx_extraction::CancelToken;
use p4kx_extraction::model::MaterialStatus;
use p4kx_settings::ModelMaterials;

use crate::assembly_export::{Stoppable, StoppableTextures, failure_of};
use crate::forge_cache::ReadFailure;
use crate::model::describe;
use crate::{
    AssemblyScene, BuildError, BuildOptions, EntryIndex, Hooks, ModelExportFailure, Progress,
};

/// The step a building view is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewStage {
    /// The models of the parts are read from the Archive.
    Reading,
    /// The models are flattened into shared meshes.
    Flattening,
    /// The textures of the materials are read.
    Textures,
}

/// How far a building view has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewProgress {
    /// The step it is at.
    pub stage: ViewStage,
    /// What is done of the step.
    pub done: u64,
    /// What the step has to do, 0 while unknown.
    pub total: u64,
}

#[derive(Debug, Default)]
pub(super) struct Shared {
    stage: AtomicU8,
    done: AtomicU64,
    total: AtomicU64,
}

impl Shared {
    fn progress(&self) -> ViewProgress {
        ViewProgress {
            stage: match self.stage.load(Ordering::Relaxed) {
                0 => ViewStage::Reading,
                1 => ViewStage::Flattening,
                _ => ViewStage::Textures,
            },
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }

    fn set(&self, stage: u8, done: u64, total: u64) {
        self.stage.store(stage, Ordering::Relaxed);
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
    }
}

/// What the view is built from.
pub(super) struct Request {
    pub(super) tree: Arc<EntityNode>,
    pub(super) lod: u32,
    pub(super) materials: ModelMaterials,
}

/// A built view and the number of parts the Assembly could not resolve.
pub(super) struct Built {
    pub(super) scene: AssemblyScene,
    pub(super) issues: usize,
}

pub(super) enum Ended {
    Built(Box<Built>),
    Cancelled,
    Empty,
    Failed(ModelExportFailure),
}

/// A view being built on a background thread; dropping it stops the work.
pub(super) struct Job {
    result: Receiver<Ended>,
    cancel: CancelToken,
    shared: Arc<Shared>,
}

impl Job {
    pub(super) fn progress(&self) -> ViewProgress {
        self.shared.progress()
    }

    pub(super) fn cancel(&self) {
        self.cancel.cancel();
    }

    pub(super) fn poll(&self) -> Option<Ended> {
        match self.result.try_recv() {
            Ok(ended) => Some(ended),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Ended::Failed(ModelExportFailure::Stopped)),
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

struct Counting<'a> {
    inner: &'a dyn ModelProvider,
    shared: &'a Shared,
}

impl ModelProvider for Counting<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        let model = self.inner.model(path, lod);
        let done = self.shared.done.fetch_add(1, Ordering::Relaxed) + 1;
        self.shared.total.store(done, Ordering::Relaxed);
        model
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.inner.library(entry, name)
    }
}

/// Starts building the view of `request` from `index`, calling `wake` from the thread once it
/// ended.
pub(super) fn start(
    index: &Arc<EntryIndex>,
    request: Request,
    wake: Arc<dyn Fn() + Send + Sync>,
) -> Result<Job, String> {
    let (sender, result) = mpsc::channel();
    let cancel = CancelToken::new();
    let shared = Arc::new(Shared::default());
    let thread_index = Arc::clone(index);
    let (thread_cancel, thread_shared) = (cancel.clone(), Arc::clone(&shared));
    thread::Builder::new()
        .name("assembly-view".to_owned())
        .spawn(move || {
            let ended = run(&thread_index, &request, &thread_cancel, &thread_shared);
            if sender.send(ended).is_ok() {
                wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Job {
        result,
        cancel,
        shared,
    })
}

fn scene_of(
    assembly: &Assembly,
    (models, materials): (&ArchiveModels<'_>, ModelMaterials),
    cancel: &CancelToken,
    shared: &Shared,
) -> Result<AssemblyScene, BuildError> {
    let archive_textures = models.textures();
    let textures = StoppableTextures {
        textures: &archive_textures,
        cancel,
    };
    let count = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
    AssemblyScene::build(
        assembly,
        &BuildOptions {
            materials,
            textures: Some(&textures),
        },
        &Hooks {
            cancelled: &|| cancel.is_cancelled(),
            progress: &|progress| match progress {
                Progress::Flattening { done, total } => shared.set(1, count(done), count(total)),
                Progress::Textures { done, total } => shared.set(2, count(done), count(total)),
            },
        },
    )
}

fn run(index: &EntryIndex, request: &Request, cancel: &CancelToken, shared: &Shared) -> Ended {
    if index.watch().check().is_err() {
        return Ended::Failed(ModelExportFailure::ArchiveChanged);
    }
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
    let stoppable = Stoppable {
        models: &archive_models,
        cancel,
    };
    let models = Counting {
        inner: &stoppable,
        shared,
    };
    let options = AssemblyOptions::default().with_lod(request.lod);
    let assembly = match Assembly::build_with_palettes(&request.tree, &models, &palettes, &options)
    {
        Ok(assembly) => assembly,
        Err(_) if cancel.is_cancelled() => return Ended::Cancelled,
        Err(AssemblyError::Provider(error)) => {
            return Ended::Failed(failure_of(index, &*error));
        }
        Err(other) => return Ended::Failed(failure_of(index, &other)),
    };
    let built = scene_of(
        &assembly,
        (&archive_models, request.materials),
        cancel,
        shared,
    );
    if index.watch().check().is_err() {
        return Ended::Failed(ModelExportFailure::ArchiveChanged);
    }
    match built {
        Ok(scene) => Ended::Built(Box::new(Built {
            scene,
            issues: assembly.issues().len(),
        })),
        Err(BuildError::Cancelled) => Ended::Cancelled,
        Err(BuildError::Empty) => Ended::Empty,
    }
}
