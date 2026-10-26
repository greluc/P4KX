// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_model::{Lod, Model};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::build::{file_key, issue};
use crate::flatten::Draft;
use crate::{AssemblyError, AssemblyOptions, Issue, IssueKind, ModelFailure, ModelProvider};

/// The most distinct model files read at once before the vertex budget is checked again; a wave
/// is twice the number of threads, at most this many.
const WAVE: usize = 256;

/// A model read for the Assembly and the file it came from.
pub(crate) struct Loaded {
    pub(crate) entry: String,
    pub(crate) model: Arc<Model>,
    pub(crate) left_out: Vec<String>,
}

/// What reading one distinct geometry file came to.
pub(crate) enum Outcome {
    Loaded(Loaded),
    Failed { missing: bool, reason: String },
}

pub(crate) type Models = HashMap<String, Outcome>;

fn vertices(model: &Model) -> usize {
    model.lods.iter().map(Lod::vertex_count).max().unwrap_or(0)
}

/// Reads every distinct geometry file the parts name, in file order a wave of files at a time in
/// parallel, and stops after the wave in which the distinct models read hold
/// `options.max_vertices` vertices. The files come in the order of their first part, so a file left
/// unread belongs to a part that [`choose_models`] refuses anyway, whatever the size of the wave;
/// the budget is overshot by one wave at most.
pub(crate) fn read_models(
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
    let mut read_vertices = 0_usize;
    let wave_size = rayon::current_num_threads()
        .saturating_mul(2)
        .clamp(1, WAVE);
    for wave in files.chunks(wave_size) {
        if read_vertices >= options.max_vertices {
            break;
        }
        let read: Vec<(String, Result<Loaded, ModelFailure>)> = wave
            .par_iter()
            .map(|(key, path)| {
                let result = provider.model(path, lod).map(|provided| Loaded {
                    entry: provided.entry,
                    model: Arc::new(provided.model),
                    left_out: provided.left_out,
                });
                (key.clone(), result)
            })
            .collect();
        for (key, result) in read {
            if let Ok(loaded) = &result {
                read_vertices = read_vertices.saturating_add(vertices(&loaded.model));
            }
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

/// Picks the model of each part in order, within the vertex budget, and records why a part has
/// none.
pub(crate) fn choose_models<'m>(
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
