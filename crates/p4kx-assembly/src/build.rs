// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_model::{Lod, MaterialStatus, Model, Transform};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::build_palettes::Palettes;
use crate::flatten::{Draft, flatten};
use crate::placement::{node_transform, offset};
use crate::{
    Assembly, AssemblyError, AssemblyOptions, EntityNode, Issue, IssueKind, ModelFailure,
    ModelProvider, PaletteProvider, Part, Placement, PortNode, PortTransform,
};

/// The most distinct model files read at once before the vertex budget is checked again.
const WAVE: usize = 256;

/// A model read for the Assembly and the file it came from.
struct Loaded {
    entry: String,
    model: Arc<Model>,
}

/// What reading one distinct geometry file came to.
enum Outcome {
    Loaded(Loaded),
    Failed { missing: bool, reason: String },
}

type Models = HashMap<String, Outcome>;
type Libraries = HashMap<String, Arc<MaterialStatus>>;

impl Assembly {
    /// Builds the Assembly of the Loadout tree `tree`: the Entity's geometry and, for every
    /// attached Entity, its geometry placed at its port.
    ///
    /// The models are read at the level of detail in `options`, each distinct file once and in
    /// parallel; a material library the Entity's geometry names replaces the one the model names,
    /// and each library is read once. A part whose Entity or geometry cannot be resolved is left
    /// out or kept without a model, and the [`Issue`] says why. The number of parts, their depth
    /// and the vertices of their models are limited by [`MAX_ASSEMBLY_PARTS`](crate::MAX_ASSEMBLY_PARTS),
    /// [`MAX_ASSEMBLY_DEPTH`](crate::MAX_ASSEMBLY_DEPTH) and [`MAX_ASSEMBLY_VERTICES`].
    ///
    /// # Errors
    ///
    /// Returns [`AssemblyError::Provider`] when `models` fails for every file, e.g. because an
    /// Archive key is missing.
    pub fn build(
        tree: &EntityNode,
        models: &dyn ModelProvider,
        options: &AssemblyOptions,
    ) -> Result<Self, AssemblyError> {
        Self::build_inner(tree, models, None, options)
    }

    /// Builds the Assembly like [`Self::build`] and resolves the tint palette of each part with
    /// `palettes`: the one its Entity's geometry names, else the default of its material
    /// library. Each distinct palette is read once and shared by the parts that use it.
    ///
    /// A palette that cannot be resolved is an [`Issue`] of the first part that names it; the
    /// part falls back to the library's default palette, or keeps the colours of its library.
    ///
    /// # Errors
    ///
    /// As [`Self::build`].
    pub fn build_with_palettes(
        tree: &EntityNode,
        models: &dyn ModelProvider,
        palettes: &dyn PaletteProvider,
        options: &AssemblyOptions,
    ) -> Result<Self, AssemblyError> {
        Self::build_inner(tree, models, Some(palettes), options)
    }

    fn build_inner(
        tree: &EntityNode,
        models: &dyn ModelProvider,
        palettes: Option<&dyn PaletteProvider>,
        options: &AssemblyOptions,
    ) -> Result<Self, AssemblyError> {
        let flat = flatten(tree, options.max_parts);
        let mut issues = flat.issues;
        let drafts = flat.drafts;
        let loaded = read_models(&drafts, models, options)?;
        let chosen = choose_models(&drafts, &loaded, options.max_vertices, &mut issues);
        let libraries = read_libraries(&drafts, &chosen, models);
        let unnamed = Arc::new(MaterialStatus::Unnamed);
        let mut reported = HashSet::new();
        let mut palettes = Palettes::new(palettes);
        let mut parts: Vec<Part> = Vec::with_capacity(drafts.len());
        for (draft, model) in drafts.iter().zip(&chosen) {
            let key = model.and_then(|loaded| library_key(draft, loaded));
            let materials = key
                .as_ref()
                .and_then(|key| libraries.get(key))
                .map_or_else(|| Arc::clone(&unnamed), Arc::clone);
            if let (Some(key), Some(reason)) = (key, materials.problem())
                && reported.insert(key)
            {
                issues.push(issue(draft, IssueKind::MaterialLibrary { reason }));
            }
            let palette = if model.is_some() {
                let (palette, unresolved) = palettes.of(draft.node.geometry(), &materials);
                issues.extend(unresolved.into_iter().map(|kind| issue(draft, kind)));
                palette
            } else {
                None
            };
            let (local, placement) = place(draft, &parts, *model, &mut issues);
            let world = draft
                .parent
                .and_then(|parent| parts.get(parent))
                .map_or(local, |parent| parent.world.then(&local));
            parts.push(Part {
                name: draft.name.clone(),
                path: draft.path.clone(),
                guid: draft.node.guid(),
                class_name: draft.node.class_name().to_owned(),
                parent: draft.parent,
                local,
                world,
                placement,
                geometry: draft.node.geometry().map(|g| g.path().to_owned()),
                model: model.map(|loaded| Arc::clone(&loaded.model)),
                materials,
                palette,
            });
        }
        Ok(Self {
            parts,
            issues,
            lod: options.lod,
        })
    }
}

fn issue(draft: &Draft<'_>, kind: IssueKind) -> Issue {
    Issue {
        path: draft.path.clone(),
        name: draft.node.class_name().to_owned(),
        kind,
    }
}

/// Returns a key for a path that ignores case, the separator kind and a leading `Data\`.
pub(crate) fn file_key(path: &str) -> String {
    let key = path
        .trim_start_matches(['/', '\\'])
        .replace('/', "\\")
        .to_ascii_lowercase();
    key.strip_prefix("data\\")
        .map_or_else(|| key.clone(), str::to_owned)
}

fn vertices(model: &Model) -> usize {
    model.lods.iter().map(Lod::vertex_count).max().unwrap_or(0)
}

/// Reads every distinct geometry file the parts name, a wave of files at a time in parallel,
/// and stops when the parts read so far already hold `options.max_vertices` vertices.
fn read_models(
    drafts: &[Draft<'_>],
    provider: &dyn ModelProvider,
    options: &AssemblyOptions,
) -> Result<Models, AssemblyError> {
    let lod = options.lod;
    let mut seen = HashSet::new();
    let files: Vec<(String, &str)> = drafts
        .iter()
        .filter_map(|draft| draft.node.geometry())
        .filter(|geometry| seen.insert(file_key(geometry.path())))
        .map(|geometry| (file_key(geometry.path()), geometry.path()))
        .collect();
    let mut models = Models::new();
    for wave in files.chunks(WAVE) {
        if budget_spent(drafts, &models, options.max_vertices) {
            break;
        }
        let read: Vec<(String, Result<Loaded, ModelFailure>)> = wave
            .par_iter()
            .map(|(key, path)| {
                let result = provider.model(path, lod).map(|provided| Loaded {
                    entry: provided.entry,
                    model: Arc::new(provided.model),
                });
                (key.clone(), result)
            })
            .collect();
        for (key, result) in read {
            let outcome = match result {
                Ok(loaded) => Outcome::Loaded(loaded),
                Err(ModelFailure::Missing) => Outcome::Failed {
                    missing: true,
                    reason: String::new(),
                },
                Err(ModelFailure::Fatal(source)) => return Err(AssemblyError::Provider(source)),
                Err(other) => Outcome::Failed {
                    missing: false,
                    reason: other.to_string(),
                },
            };
            models.insert(key, outcome);
        }
    }
    Ok(models)
}

fn budget_spent(drafts: &[Draft<'_>], models: &Models, max_vertices: usize) -> bool {
    let total: usize = drafts
        .iter()
        .filter_map(|draft| draft.node.geometry())
        .filter_map(|geometry| match models.get(&file_key(geometry.path())) {
            Some(Outcome::Loaded(loaded)) => Some(vertices(&loaded.model)),
            _ => None,
        })
        .fold(0, usize::saturating_add);
    total >= max_vertices
}

/// Picks the model of each part in order, within the vertex budget, and records why a part has
/// none.
fn choose_models<'m>(
    drafts: &[Draft<'_>],
    models: &'m Models,
    max_vertices: usize,
    issues: &mut Vec<Issue>,
) -> Vec<Option<&'m Loaded>> {
    let mut total = 0_usize;
    drafts
        .iter()
        .map(|draft| {
            let Some(geometry) = draft.node.geometry() else {
                issues.push(issue(draft, IssueKind::NoGeometry));
                return None;
            };
            let path = geometry.path().to_owned();
            match models.get(&file_key(&path)) {
                Some(Outcome::Loaded(loaded)) => {
                    let count = vertices(&loaded.model);
                    if total.saturating_add(count) > max_vertices {
                        let limit = max_vertices;
                        issues.push(issue(draft, IssueKind::TooManyVertices { limit }));
                        None
                    } else {
                        total += count;
                        Some(loaded)
                    }
                }
                Some(Outcome::Failed { missing: true, .. }) => {
                    issues.push(issue(draft, IssueKind::GeometryMissing { path }));
                    None
                }
                Some(Outcome::Failed { reason, .. }) => {
                    let reason = reason.clone();
                    issues.push(issue(draft, IssueKind::GeometryUnreadable { path, reason }));
                    None
                }
                None => {
                    let limit = max_vertices;
                    issues.push(issue(draft, IssueKind::TooManyVertices { limit }));
                    None
                }
            }
        })
        .collect()
}

/// Returns the name of the library a part draws with: the one its Entity chooses, else the one
/// its model names.
fn library_name<'a>(draft: &'a Draft<'_>, loaded: &'a Loaded) -> Option<&'a str> {
    draft
        .node
        .geometry()
        .and_then(|geometry| geometry.material())
        .or(loaded.model.material_library.as_deref())
}

/// Returns the key under which a part's library is cached: a bare name is relative to the
/// model's folder, any other name is the same for every model.
fn library_key(draft: &Draft<'_>, loaded: &Loaded) -> Option<String> {
    Some(library_key_of(library_name(draft, loaded)?, &loaded.entry))
}

/// Returns the key under which the library `name` of the model at `entry` is cached.
pub(crate) fn library_key_of(name: &str, entry: &str) -> String {
    let key = file_key(name);
    let key = key.strip_suffix(".mtl").unwrap_or(&key).to_owned();
    if key.contains('\\') {
        key
    } else {
        let folder = entry
            .rsplit_once(['\\', '/'])
            .map_or("", |(folder, _)| folder);
        format!("{}\\{key}", folder.to_ascii_lowercase())
    }
}

/// Reads every distinct material library the parts draw with once, in parallel.
fn read_libraries(
    drafts: &[Draft<'_>],
    chosen: &[Option<&Loaded>],
    provider: &dyn ModelProvider,
) -> Libraries {
    let mut seen = HashSet::new();
    let wanted: Vec<(String, &str, &str)> = drafts
        .iter()
        .zip(chosen)
        .filter_map(|(draft, loaded)| {
            let loaded = (*loaded)?;
            let name = library_name(draft, loaded)?;
            let key = library_key(draft, loaded)?;
            seen.insert(key.clone())
                .then_some((key, loaded.entry.as_str(), name))
        })
        .collect();
    wanted
        .par_iter()
        .map(|(key, entry, name)| (key.clone(), Arc::new(provider.library(entry, name))))
        .collect()
}

/// Returns the transform of a part to its parent's and how it was found.
fn place(
    draft: &Draft<'_>,
    parts: &[Part],
    model: Option<&Loaded>,
    issues: &mut Vec<Issue>,
) -> (Transform, Placement) {
    let Some(parent) = draft.parent.and_then(|parent| parts.get(parent)) else {
        return (Transform::IDENTITY, Placement::Root);
    };
    let Some(port) = draft.port else {
        return (Transform::IDENTITY, Placement::Origin);
    };
    let definition = port
        .definition()
        .and_then(|definition| definition.transform());
    let helper = definition
        .map(PortTransform::helper)
        .filter(|helper| !helper.is_empty());
    let (placement, base) = anchor(parent.model(), helper, port);
    if let Some(helper) = helper
        && placement == Placement::Origin
    {
        let helper = helper.to_owned();
        issues.push(issue(draft, IssueKind::HelperNotFound { helper }));
    }
    let Some(definition) = definition else {
        return (base, placement);
    };
    let mut local = base.then(&offset(definition.position(), definition.rotation()));
    let inner = (!definition.offset_helper().is_empty())
        .then(|| {
            let child = model?;
            node_transform(&child.model, definition.offset_helper())?.inverse()
        })
        .flatten();
    if let Some(inner) = inner {
        local = local.then(&inner);
    }
    (local, placement)
}

/// Finds the node of the parent's model a port is anchored to: its helper, else a node named
/// like the port.
fn anchor(parent: Option<&Model>, helper: Option<&str>, port: &PortNode) -> (Placement, Transform) {
    let Some(parent) = parent else {
        return (Placement::Origin, Transform::IDENTITY);
    };
    if let Some(helper) = helper
        && let Some(transform) = node_transform(parent, helper)
    {
        return (Placement::Helper(helper.to_owned()), transform);
    }
    node_transform(parent, port.name())
        .map_or((Placement::Origin, Transform::IDENTITY), |transform| {
            (Placement::PortNode, transform)
        })
}
