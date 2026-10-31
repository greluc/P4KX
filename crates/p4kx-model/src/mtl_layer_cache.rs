// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use crate::mtl_load::request_path;
use crate::{CompanionPath, CompanionResolver, LayerSurface, Material, MaterialLibrary};

/// The most distinct layer files one load of a material library may fetch; layers beyond are
/// reported as unresolved.
pub const MAX_LAYER_FILES: usize = 256;

/// The most bytes of layer files one load of a material library may fetch; layers beyond are
/// reported as unresolved.
pub const MAX_LAYER_BYTES: usize = 64 * 1024 * 1024;

/// The most layer files a [`LayerCache`] keeps; it starts over when it is full.
const MAX_CACHED_FILES: usize = 4 * MAX_LAYER_FILES;

/// What one load of a material library may still fetch.
#[derive(Debug)]
pub(crate) struct Budget {
    files: usize,
    bytes: usize,
}

impl Budget {
    pub(crate) const fn new() -> Self {
        Self {
            files: MAX_LAYER_FILES,
            bytes: MAX_LAYER_BYTES,
        }
    }

    const fn allows_fetch(&self) -> bool {
        self.files > 0 && self.bytes > 0
    }

    const fn spend(&mut self, bytes: usize) {
        self.files = self.files.saturating_sub(1);
        self.bytes = self.bytes.saturating_sub(bytes);
    }
}

/// The surfaces of the sub-materials of one layer file; the parsed file is dropped.
#[derive(Debug)]
struct LayerFile {
    single: bool,
    surfaces: Vec<(String, LayerSurface)>,
}

impl LayerFile {
    fn of(library: &MaterialLibrary) -> Self {
        Self {
            single: library.single,
            surfaces: library
                .materials
                .iter()
                .map(|material| (material.name.clone(), LayerSurface::of(material)))
                .collect(),
        }
    }

    fn pick(&self, submaterial: &str) -> Option<&LayerSurface> {
        let at = pick_position(
            self.single,
            submaterial,
            self.surfaces.iter().map(|(name, _)| name.as_str()),
        )?;
        self.surfaces.get(at).map(|(_, surface)| surface)
    }
}

/// Returns the position among `names` of the sub-material a layer's `Submtl` names: the first
/// for a single material file or an empty `submaterial`, else the one with that name, ignoring
/// case.
fn pick_position<'a>(
    single: bool,
    submaterial: &str,
    mut names: impl Iterator<Item = &'a str>,
) -> Option<usize> {
    if single || submaterial.is_empty() {
        names.next().map(|_| 0)
    } else {
        names.position(|name| name.eq_ignore_ascii_case(submaterial))
    }
}

impl MaterialLibrary {
    /// Returns the position of the sub-material the layer attribute `Submtl` names: the first
    /// one when the file is a single material or the name is empty, else the one with that name,
    /// ignoring case; `None` when there is none.
    #[must_use]
    pub fn layer_position(&self, submaterial: &str) -> Option<usize> {
        pick_position(
            self.single,
            submaterial.trim(),
            self.materials.iter().map(|material| material.name.as_str()),
        )
    }
}

impl LayerSurface {
    pub(crate) fn of(material: &Material) -> Self {
        Self {
            diffuse: material.diffuse,
            specular: material.specular,
            textures: material.textures.clone(),
        }
    }
}

/// The layer files read so far, as their surfaces.
///
/// The loads of several material libraries share it, so that a file many libraries layer in is
/// fetched and parsed once; it keeps the surfaces of the files, not their parsed materials.
#[derive(Debug, Default)]
pub struct LayerCache {
    files: HashMap<String, Option<LayerFile>>,
}

impl LayerCache {
    /// Returns an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the surface of the sub-material `submaterial` of the layer file at `path`, `None`
    /// when the path is unsafe, the file is missing or unreadable, has no such sub-material, or
    /// `budget` is spent before the file could be fetched.
    pub(crate) fn surface(
        &mut self,
        path: &str,
        submaterial: &str,
        companions: &mut dyn CompanionResolver,
        budget: &mut Budget,
    ) -> Option<LayerSurface> {
        let request = request_path(path)?;
        let key = request.to_ascii_lowercase();
        if !self.files.contains_key(&key) {
            if !budget.allows_fetch() {
                return None;
            }
            if self.files.len() >= MAX_CACHED_FILES {
                self.files.clear();
            }
            let file = fetch(&request, companions, budget);
            self.files.insert(key.clone(), file);
        }
        self.files.get(&key)?.as_ref()?.pick(submaterial).cloned()
    }
}

fn fetch(
    request: &str,
    companions: &mut dyn CompanionResolver,
    budget: &mut Budget,
) -> Option<LayerFile> {
    let bytes = companions
        .resolve(CompanionPath::Material(request))
        .ok()
        .flatten();
    budget.spend(bytes.as_ref().map_or(0, Vec::len));
    let library = MaterialLibrary::parse(request, &bytes?).ok()?;
    Some(LayerFile::of(&library))
}
