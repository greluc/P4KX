// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_model::{Lod, MaterialStatus, Model};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::InteriorOptions;
use super::walk::Walked;
use crate::build::{file_key, library_key_of};
use crate::{ArchiveModels, AssemblyError, Issue, IssueKind, ModelFailure, ModelProvider};

/// The most distinct model files read at once before the vertex budget is checked again.
const WAVE: usize = 256;

type Key = (usize, String);

struct Loaded {
    entry: String,
    model: Arc<Model>,
}

enum Outcome {
    Loaded(Loaded),
    Failed { missing: bool, reason: String },
    NotRead,
}

fn vertices(model: &Model) -> usize {
    model.lods.iter().map(Lod::vertex_count).max().unwrap_or(0)
}

/// Reads the model of every model part and the material library it draws with, each distinct
/// file once and in parallel, looking in the part's container first and then in the containers
/// around it; fills the parts in and lists what could not be read.
pub(super) fn attach(
    walked: &mut Walked<'_>,
    options: &InteriorOptions,
) -> Result<(), AssemblyError> {
    let providers: Vec<ArchiveModels<'_>> = (0..walked.layers.len())
        .filter_map(|layer| walked.layers.get(layer))
        .map(ArchiveModels::new)
        .collect();
    let resolved = resolve(walked, &providers);
    let mut files: Vec<(Key, String)> = Vec::new();
    let mut seen = HashSet::new();
    for (key, path) in resolved.iter().flatten() {
        if seen.insert(key.clone()) {
            files.push((key.clone(), path.clone()));
        }
    }
    let outcomes = read_models(&files, &providers, options)?;
    let libraries = read_libraries(&outcomes, &providers);
    let unnamed = Arc::new(MaterialStatus::Unnamed);
    let mut reported: HashSet<(String, String)> = HashSet::new();
    let mut issues: Vec<Issue> = Vec::new();
    let mut note = |container: &str, name: &str, kind: IssueKind| {
        if reported.insert((container.to_owned(), format!("{name}|{kind}"))) {
            issues.push(Issue {
                path: container.to_owned(),
                name: name.to_owned(),
                kind,
            });
        }
    };
    for (placement, found) in walked.placements.iter().zip(&resolved) {
        let Some(part) = walked.parts.get_mut(placement.part) else {
            continue;
        };
        let path = part.model_path.clone().unwrap_or_default();
        let name = part.name.clone();
        let Some((key, _)) = found else {
            note(
                &placement.container,
                &name,
                IssueKind::GeometryMissing { path },
            );
            continue;
        };
        match outcomes.get(key) {
            Some(Outcome::Loaded(loaded)) => {
                part.model = Some(Arc::clone(&loaded.model));
                let library = library_key(key.0, loaded);
                part.materials = library
                    .as_ref()
                    .and_then(|key| libraries.get(key))
                    .map_or_else(|| Arc::clone(&unnamed), Arc::clone);
                if let (Some(_), Some(reason)) = (library, part.materials.problem()) {
                    note(
                        &placement.container,
                        &name,
                        IssueKind::MaterialLibrary { reason },
                    );
                }
            }
            Some(Outcome::Failed { missing: true, .. }) => {
                note(
                    &placement.container,
                    &name,
                    IssueKind::GeometryMissing { path },
                );
            }
            Some(Outcome::Failed { reason, .. }) => {
                let reason = reason.clone();
                note(
                    &placement.container,
                    &name,
                    IssueKind::GeometryUnreadable { path, reason },
                );
            }
            Some(Outcome::NotRead) | None => {
                let limit = options.max_vertices;
                note(
                    &placement.container,
                    &name,
                    IssueKind::TooManyVertices { limit },
                );
            }
        }
    }
    walked.issues.extend(issues);
    Ok(())
}

/// Returns for every placement the layer that holds its model and the path as the part spells it;
/// `None` when no layer holds it.
fn resolve(walked: &Walked<'_>, providers: &[ArchiveModels<'_>]) -> Vec<Option<(Key, String)>> {
    let mut cache: HashMap<(usize, String), Option<usize>> = HashMap::new();
    walked
        .placements
        .iter()
        .map(|placement| {
            let part = walked.parts.get(placement.part)?;
            let path = part.model_path.as_deref()?;
            let file = file_key(path);
            let last = placement.chain.last().copied().unwrap_or_default();
            let layer = *cache.entry((last, file.clone())).or_insert_with(|| {
                placement
                    .chain
                    .iter()
                    .rev()
                    .copied()
                    .find(|&layer| providers.get(layer).is_some_and(|p| p.contains(path)))
            });
            Some(((layer?, file), path.to_owned()))
        })
        .collect()
}

fn read_models(
    files: &[(Key, String)],
    providers: &[ArchiveModels<'_>],
    options: &InteriorOptions,
) -> Result<HashMap<Key, Outcome>, AssemblyError> {
    let mut outcomes: HashMap<Key, Outcome> = HashMap::new();
    let mut total = 0_usize;
    for wave in files.chunks(WAVE) {
        if total >= options.max_vertices {
            break;
        }
        let read: Vec<(Key, Result<(String, Model), ModelFailure>)> = wave
            .par_iter()
            .map(|(key, path)| {
                let result = providers
                    .get(key.0)
                    .ok_or(ModelFailure::Missing)
                    .and_then(|provider| provider.model(path, options.lod))
                    .map(|provided| (provided.entry, provided.model));
                (key.clone(), result)
            })
            .collect();
        for (key, result) in read {
            let outcome = match result {
                Ok((entry, model)) => {
                    let count = vertices(&model);
                    if total.saturating_add(count) > options.max_vertices {
                        Outcome::NotRead
                    } else {
                        total += count;
                        Outcome::Loaded(Loaded {
                            entry,
                            model: Arc::new(model),
                        })
                    }
                }
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
            outcomes.insert(key, outcome);
        }
    }
    Ok(outcomes)
}

fn library_key(layer: usize, loaded: &Loaded) -> Option<Key> {
    let name = loaded.model.material_library.as_deref()?;
    Some((layer, library_key_of(name, &loaded.entry)))
}

fn read_libraries(
    outcomes: &HashMap<Key, Outcome>,
    providers: &[ArchiveModels<'_>],
) -> HashMap<Key, Arc<MaterialStatus>> {
    let mut seen = HashSet::new();
    let wanted: Vec<(Key, &str, &str)> = outcomes
        .iter()
        .filter_map(|(key, outcome)| match outcome {
            Outcome::Loaded(loaded) => {
                let library = library_key(key.0, loaded)?;
                let name = loaded.model.material_library.as_deref()?;
                seen.insert(library.clone())
                    .then_some((library, loaded.entry.as_str(), name))
            }
            _ => None,
        })
        .collect();
    wanted
        .par_iter()
        .filter_map(|(key, entry, name)| {
            let provider = providers.get(key.0)?;
            Some((key.clone(), Arc::new(provider.library(entry, name))))
        })
        .collect()
}
