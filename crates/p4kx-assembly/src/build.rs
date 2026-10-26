// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_model::{MaterialStatus, Model, Transform};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::build_models::{Loaded, choose_models, fall_back, read_models};
use crate::build_palettes::Palettes;
use crate::flatten::{Draft, flatten};
use crate::placement::{node_transform, offset};
use crate::{
    Assembly, AssemblyError, AssemblyOptions, EntityNode, Issue, IssueKind, ModelProvider,
    ObjectContainerRef, PaletteProvider, Part, Placement, PortNode, PortTransform,
};

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
        let mut drafts = flat.drafts;
        let mut loaded = read_models(&drafts, models, options)?;
        fall_back(&mut drafts, &mut loaded, models, options, &mut issues)?;
        let chosen = choose_models(&drafts, &loaded, options.max_vertices, &mut issues);
        let libraries = read_libraries(&drafts, &chosen, models);
        let unnamed = Arc::new(MaterialStatus::Unnamed);
        let mut reported = HashSet::new();
        let mut characters = HashSet::new();
        let mut palettes = Palettes::new(palettes);
        let mut parts: Vec<Part> = Vec::with_capacity(drafts.len());
        for (draft, model) in drafts.iter().zip(&chosen) {
            let key = model.and_then(|loaded| library_key(draft, loaded));
            let materials = key
                .as_ref()
                .and_then(|key| libraries.get(key))
                .map_or_else(|| Arc::clone(&unnamed), Arc::clone);
            if let (Some(key), Some(reason)) = (
                key,
                materials.problem().or_else(|| materials.layer_problem()),
            ) && reported.insert(key)
            {
                issues.push(issue(draft, IssueKind::MaterialLibrary { reason }));
            }
            note_attachments(draft, *model, &mut characters, &mut issues);
            let palette = if model.is_some() {
                let (palette, unresolved) = palettes.of(draft.geometry, &materials);
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
                class_name: draft.class_name().to_owned(),
                parent: draft.parent,
                local,
                world,
                placement,
                geometry: draft.geometry_path().map(str::to_owned),
                model: model.map(|loaded| Arc::clone(&loaded.model)),
                materials,
                palette,
            });
        }
        Ok(Self {
            parts,
            issues,
            lod: options.lod,
            interior: None,
            containers: containers_of(&drafts),
        })
    }
}

/// Returns the object containers each Entity of `drafts` names, shared with its node; a docked
/// vehicle that is not expanded names none, as its own ports are left out too.
fn containers_of(drafts: &[Draft<'_>]) -> Vec<Arc<[ObjectContainerRef]>> {
    let none: Arc<[ObjectContainerRef]> = Arc::new([]);
    drafts
        .iter()
        .map(|draft| {
            if draft.gear.is_some() || (draft.node.is_docked_vehicle() && !draft.node.is_expanded())
            {
                Arc::clone(&none)
            } else {
                Arc::clone(&draft.node.object_containers)
            }
        })
        .collect()
}

/// Lists the attachments of a character definition that are not drawn, once per definition.
fn note_attachments(
    draft: &Draft<'_>,
    model: Option<&Loaded>,
    seen: &mut HashSet<String>,
    issues: &mut Vec<Issue>,
) {
    if let (Some(loaded), Some(path)) = (model, draft.geometry_path())
        && !loaded.left_out.is_empty()
        && seen.insert(file_key(path))
    {
        let files = loaded.left_out.clone();
        issues.push(issue(draft, IssueKind::AttachmentsLeftOut { files }));
    }
}

pub(crate) fn issue(draft: &Draft<'_>, kind: IssueKind) -> Issue {
    Issue {
        path: draft.path.clone(),
        name: draft.class_name().to_owned(),
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

/// Returns the name of the library a part draws with: the one its Entity chooses, else the one
/// its model names.
fn library_name<'a>(draft: &'a Draft<'_>, loaded: &'a Loaded) -> Option<&'a str> {
    draft
        .geometry
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
    if let Some(gear) = draft.gear {
        return place_gear(draft, gear.bone(), parent, issues);
    }
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
        && parent.model().is_some()
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

/// Returns the transform of a landing gear: the node `bone` of its Entity's model.
fn place_gear(
    draft: &Draft<'_>,
    bone: &str,
    parent: &Part,
    issues: &mut Vec<Issue>,
) -> (Transform, Placement) {
    let Some(parent_model) = parent.model() else {
        return (Transform::IDENTITY, Placement::Origin);
    };
    if let Some(transform) = node_transform(parent_model, bone) {
        return (transform, Placement::Helper(bone.to_owned()));
    }
    let helper = bone.to_owned();
    issues.push(issue(draft, IssueKind::HelperNotFound { helper }));
    (Transform::IDENTITY, Placement::Origin)
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
