// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rayon::prelude::*;

use crate::texture_loader::{LoadedTexture, TextureLoadError, TextureLoader};
use crate::texture_plan::Job;

/// The most pixels the decoded sources kept for the next chunk of jobs may hold together.
pub(crate) const MAX_HELD_PIXELS: u64 = 1 << 27;

/// A decoded source texture, or why it could not be loaded.
pub(crate) type Loaded = Result<Arc<LoadedTexture>, TextureLoadError>;

/// The decoded sources one chunk of jobs is made from, by path.
pub(crate) type Decoded = HashMap<String, Loaded>;

/// Returns the paths of the source textures `job` loads.
pub(crate) fn paths(job: &Job) -> impl Iterator<Item = &str> {
    [&job.diffuse, &job.normal, &job.specular]
        .into_iter()
        .flatten()
        .map(|source| source.path.as_str())
}

/// Decodes the source textures of a build a chunk of jobs at a time, each once per chunk however
/// many jobs of it use it, and keeps those the next chunk needs while they fit a pixel cap.
pub(crate) struct SourceStore<'l> {
    loader: &'l dyn TextureLoader,
    mip: u32,
    cap: u64,
    held: Decoded,
}

fn pixels(loaded: &Loaded) -> u64 {
    loaded.as_ref().map_or(0, |texture| {
        u64::from(texture.image.width()) * u64::from(texture.image.height())
    })
}

impl<'l> SourceStore<'l> {
    /// Creates a store that loads at mip `mip` with `loader` and holds at most `cap` pixels
    /// between chunks.
    pub(crate) fn new(loader: &'l dyn TextureLoader, mip: u32, cap: u64) -> Self {
        Self {
            loader,
            mip,
            cap,
            held: HashMap::new(),
        }
    }

    /// Returns the decoded sources of `jobs`, taking the held ones and decoding the rest in
    /// parallel.
    pub(crate) fn prepare<'a>(&mut self, jobs: impl IntoIterator<Item = &'a Job>) -> Decoded {
        let mut decoded = Decoded::new();
        let mut missing: Vec<&str> = Vec::new();
        for path in jobs.into_iter().flat_map(paths) {
            if decoded.contains_key(path) || missing.contains(&path) {
                continue;
            }
            match self.held.remove_entry(path) {
                Some((path, held)) => {
                    decoded.insert(path, held);
                }
                None => missing.push(path),
            }
        }
        self.held.clear();
        let (loader, mip) = (self.loader, self.mip);
        let loaded: Vec<(String, Loaded)> = missing
            .par_iter()
            .map(|path| ((*path).to_owned(), loader.load(path, mip).map(Arc::new)))
            .collect();
        decoded.extend(loaded);
        decoded
    }

    /// Keeps the sources of `decoded` that `next` uses, as many as fit the cap.
    pub(crate) fn keep<'a>(&mut self, decoded: Decoded, next: impl IntoIterator<Item = &'a Job>) {
        let wanted: HashSet<&str> = next.into_iter().flat_map(paths).collect();
        let mut held_pixels = 0u64;
        for (path, loaded) in decoded {
            let size = pixels(&loaded);
            if wanted.contains(path.as_str()) && held_pixels.saturating_add(size) <= self.cap {
                held_pixels += size;
                self.held.insert(path, loaded);
            }
        }
    }

    /// Returns the pixels the held sources hold.
    #[cfg(test)]
    pub(crate) fn held_pixels(&self) -> u64 {
        self.held.values().map(pixels).sum()
    }

    /// Returns the size of the texture at `path` without decoding it, when the loader knows.
    pub(crate) fn dimensions(&self, path: &str) -> Result<Option<(u32, u32)>, TextureLoadError> {
        self.loader.dimensions(path, self.mip)
    }
}
