// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_model::{Lod, MaterialStatus, Model};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::walk::Walked;
use super::{InteriorOptions, InteriorPart, InteriorProgress, InteriorWatch};
use crate::build::{file_key, library_key_of};
use crate::{ArchiveModels, AssemblyError, Issue, IssueKind, ModelFailure, ModelProvider};

/// The most distinct model files read at once before the vertex budget is checked again.
const WAVE: usize = 256;

type Key = (usize, String);
type Read = Result<(String, Model), ModelFailure>;

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
    watch: &InteriorWatch<'_>,
) -> Result<(), AssemblyError> {
    let providers: Vec<ArchiveModels<'_>> = (0..walked.layers.len())
        .filter_map(|layer| walked.layers.get(layer))
        .map(ArchiveModels::new)
        .collect();
    let resolved = resolve(walked, &providers, watch)?;
    let mut files: Vec<(Key, String)> = Vec::new();
    let mut seen = HashSet::new();
    for (key, path) in resolved.iter().flatten() {
        if seen.insert(key.clone()) {
            files.push((key.clone(), path.clone()));
        }
    }
    let outcomes = read_models(&files, &providers, (options, watch))?;
    let libraries = read_libraries(&outcomes, &providers, watch)?;
    let mut fill = Fill {
        outcomes,
        libraries,
        unnamed: Arc::new(MaterialStatus::Unnamed),
        max_vertices: options.max_vertices,
        reported: HashSet::new(),
        issues: Vec::new(),
    };
    for (placement, found) in walked.placements.iter().zip(&resolved) {
        if watch.is_cancelled() {
            return Err(AssemblyError::Cancelled);
        }
        if let Some(part) = walked.parts.get_mut(placement.part) {
            fill.part(
                part,
                &placement.container,
                found.as_ref().map(|(key, _)| key),
            );
        }
    }
    walked.issues.extend(fill.issues);
    Ok(())
}

/// The models and libraries that were read, and the issues found while handing them to parts.
struct Fill {
    outcomes: HashMap<Key, Outcome>,
    libraries: HashMap<Key, Arc<MaterialStatus>>,
    unnamed: Arc<MaterialStatus>,
    max_vertices: usize,
    reported: HashSet<(String, String)>,
    issues: Vec<Issue>,
}

impl Fill {
    fn note(&mut self, container: &str, name: &str, kind: IssueKind) {
        if self
            .reported
            .insert((container.to_owned(), format!("{name}|{kind}")))
        {
            self.issues.push(Issue {
                path: container.to_owned(),
                name: name.to_owned(),
                kind,
            });
        }
    }

    fn part(&mut self, part: &mut InteriorPart, container: &str, key: Option<&Key>) {
        let path = part.model_path.clone().unwrap_or_default();
        let name = part.name.clone();
        let Some(key) = key else {
            self.note(container, &name, IssueKind::GeometryMissing { path });
            return;
        };
        let kind = match self.outcomes.get(key) {
            Some(Outcome::Loaded(loaded)) => {
                part.model = Some(Arc::clone(&loaded.model));
                let library = library_key(key.0, loaded);
                part.materials = library
                    .as_ref()
                    .and_then(|key| self.libraries.get(key))
                    .map_or_else(|| Arc::clone(&self.unnamed), Arc::clone);
                let problem = part
                    .materials
                    .problem()
                    .or_else(|| part.materials.layer_problem());
                match (library, problem) {
                    (Some(_), Some(reason)) => IssueKind::MaterialLibrary { reason },
                    _ => return,
                }
            }
            Some(Outcome::Failed { missing: true, .. }) => IssueKind::GeometryMissing { path },
            Some(Outcome::Failed { reason, .. }) => IssueKind::GeometryUnreadable {
                path,
                reason: reason.clone(),
            },
            Some(Outcome::NotRead) | None => IssueKind::TooManyVertices {
                limit: self.max_vertices,
            },
        };
        self.note(container, &name, kind);
    }
}

/// Returns for every placement the layer that holds its model and the path as the part spells it;
/// `None` when no layer holds it.
fn resolve(
    walked: &Walked<'_>,
    providers: &[ArchiveModels<'_>],
    watch: &InteriorWatch<'_>,
) -> Result<Vec<Option<(Key, String)>>, AssemblyError> {
    let mut cache: HashMap<(usize, String), Option<usize>> = HashMap::new();
    walked
        .placements
        .iter()
        .map(|placement| {
            if watch.is_cancelled() {
                return Err(AssemblyError::Cancelled);
            }
            let Some(path) = walked
                .parts
                .get(placement.part)
                .and_then(|part| part.model_path.as_deref())
            else {
                return Ok(None);
            };
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
            Ok(layer.map(|layer| ((layer, file), path.to_owned())))
        })
        .collect()
}

/// Reads the models of `wave` in parallel; once `watch` asks to stop, the files not yet read come
/// back as missing, for the caller to drop.
fn read_wave(
    wave: &[(Key, String)],
    providers: &[ArchiveModels<'_>],
    (options, watch): (&InteriorOptions, &InteriorWatch<'_>),
) -> Vec<(Key, Read)> {
    wave.par_iter()
        .map(|(key, path)| {
            if watch.is_cancelled() {
                return (key.clone(), Err(ModelFailure::Missing));
            }
            let result = providers
                .get(key.0)
                .ok_or(ModelFailure::Missing)
                .and_then(|provider| provider.model(path, options.lod))
                .map(|provided| (provided.entry, provided.model));
            (key.clone(), result)
        })
        .collect()
}

fn read_models(
    files: &[(Key, String)],
    providers: &[ArchiveModels<'_>],
    (options, watch): (&InteriorOptions, &InteriorWatch<'_>),
) -> Result<HashMap<Key, Outcome>, AssemblyError> {
    let mut outcomes: HashMap<Key, Outcome> = HashMap::new();
    let mut total = 0_usize;
    for wave in files.chunks(WAVE) {
        if watch.is_cancelled() {
            return Err(AssemblyError::Cancelled);
        }
        watch.report(InteriorProgress::Models {
            done: outcomes.len(),
            total: files.len(),
        });
        if total >= options.max_vertices {
            break;
        }
        for (key, result) in read_wave(wave, providers, (options, watch)) {
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
        if watch.is_cancelled() {
            return Err(AssemblyError::Cancelled);
        }
    }
    watch.report(InteriorProgress::Models {
        done: files.len(),
        total: files.len(),
    });
    Ok(outcomes)
}

fn library_key(layer: usize, loaded: &Loaded) -> Option<Key> {
    let name = loaded.model.material_library.as_deref()?;
    Some((layer, library_key_of(name, &loaded.entry)))
}

fn read_libraries(
    outcomes: &HashMap<Key, Outcome>,
    providers: &[ArchiveModels<'_>],
    watch: &InteriorWatch<'_>,
) -> Result<HashMap<Key, Arc<MaterialStatus>>, AssemblyError> {
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
    let libraries = wanted
        .par_iter()
        .filter_map(|(key, entry, name)| {
            if watch.is_cancelled() {
                return None;
            }
            let provider = providers.get(key.0)?;
            Some((key.clone(), Arc::new(provider.library(entry, name))))
        })
        .collect();
    if watch.is_cancelled() {
        return Err(AssemblyError::Cancelled);
    }
    Ok(libraries)
}
