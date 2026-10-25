// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::texture_loader::{LoadedTexture, TextureLoadError, TextureLoader};
use crate::texture_plan::Job;

type Loaded = Result<Arc<LoadedTexture>, TextureLoadError>;

struct Entry {
    uses: usize,
    cell: Arc<OnceLock<Loaded>>,
}

/// The decoded source textures of one build, each decoded once however many jobs use it and
/// dropped when the last job that uses it is done.
pub(crate) struct DecodedCache<'l> {
    loader: &'l dyn TextureLoader,
    mip: u32,
    entries: Mutex<HashMap<String, Entry>>,
}

fn paths(job: &Job) -> impl Iterator<Item = &str> {
    [&job.diffuse, &job.normal, &job.specular]
        .into_iter()
        .flatten()
        .map(|source| source.path.as_str())
}

impl<'l> DecodedCache<'l> {
    /// Creates a cache for the textures `jobs` use, loaded at mip `mip` by `loader`.
    pub(crate) fn new(jobs: &[Job], mip: u32, loader: &'l dyn TextureLoader) -> Self {
        let mut entries: HashMap<String, Entry> = HashMap::new();
        for path in jobs.iter().flat_map(paths) {
            entries
                .entry(path.to_owned())
                .or_insert_with(|| Entry {
                    uses: 0,
                    cell: Arc::new(OnceLock::new()),
                })
                .uses += 1;
        }
        Self {
            loader,
            mip,
            entries: Mutex::new(entries),
        }
    }

    /// Returns the texture at `path`, decoding it on the first call.
    pub(crate) fn get(&self, path: &str) -> Loaded {
        let cell = self
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
            .map(|entry| Arc::clone(&entry.cell));
        let load = || self.loader.load(path, self.mip).map(Arc::new);
        cell.map_or_else(load, |cell| cell.get_or_init(load).clone())
    }

    /// Returns the size of the texture at `path` without decoding it, when the loader knows.
    pub(crate) fn dimensions(&self, path: &str) -> Result<Option<(u32, u32)>, TextureLoadError> {
        self.loader.dimensions(path, self.mip)
    }

    /// Notes that `job` is done with its textures, dropping each no other job needs.
    pub(crate) fn release(&self, job: &Job) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        for path in paths(job) {
            if let Some(entry) = entries.get_mut(path) {
                entry.uses = entry.uses.saturating_sub(1);
                if entry.uses == 0 {
                    entries.remove(path);
                }
            }
        }
    }
}
