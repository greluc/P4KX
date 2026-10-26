// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use p4kx_gltf::MorphTarget;
use p4kx_model::{Lod, MaterialStatus, Model};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::face_build::{Face, Found, settled};
use super::head_move::{HeadMove, Worn, followed};
use super::jobs::{Job, issue, planned};
use super::shape::shaped;
use super::zones::{covered, without_covered};
use super::{Armour, ArmourPart, BodyVariant, Piece};
use crate::build::file_key;
use crate::{
    AssemblyError, AssemblyOptions, Issue, IssueKind, ModelFailure, ModelProvider, ProvidedModel,
};

/// The most distinct model files read at once before the vertex budget is checked again.
const WAVE: usize = 64;

/// The name of the piece the pieces that follow the head follow.
const HEAD: &str = "head";

struct Loaded {
    entry: String,
    model: Arc<Model>,
}

type Models = HashMap<String, Result<Loaded, ModelFailure>>;
type Libraries = HashMap<String, Arc<MaterialStatus>>;

struct Body {
    skeleton: Model,
    skeleton_file: String,
}

impl Armour {
    /// Builds the Armour of `pieces` worn on the default body of `variant`.
    ///
    /// The body is the skeleton of the variant and its default skin, see [`BodyVariant`]. Each
    /// piece contributes the geometry its Record names for the variant's tag, see
    /// [`Wearable`](crate::Wearable). The models are read at the level of detail in `options`,
    /// each distinct file once and in parallel, and the material library the Record names
    /// replaces the one a model names. A piece that cannot be fitted to the variant, or whose
    /// model cannot be read, is left out and the [`Issue`] says why; the rest is still built.
    /// `options.max_parts` and `options.max_vertices` limit the parts and their vertices.
    ///
    /// # Errors
    ///
    /// Returns [`AssemblyError::Body`] when the skeleton of the variant cannot be loaded and
    /// [`AssemblyError::Provider`] when `models` fails for every file, e.g. because an Archive key
    /// is missing.
    pub fn build(
        variant: BodyVariant,
        pieces: &[Piece],
        models: &dyn ModelProvider,
        options: &AssemblyOptions,
    ) -> Result<Self, AssemblyError> {
        let body = load_body(variant, models, options.lod)?;
        let mut issues = Vec::new();
        let jobs = planned(variant, pieces, models, options.max_parts, &mut issues);
        let loaded = read_models(&jobs, models, options)?;
        let chosen = choose(&jobs, &loaded, options.max_vertices, &mut issues);
        let libraries = read_libraries(&jobs, &chosen, models);
        let unnamed = Arc::new(MaterialStatus::Unnamed);
        let covering = covering(&jobs, &chosen);
        let mut reported = HashSet::new();
        let mut parts = Vec::with_capacity(chosen.len());
        let mut head: Option<HeadMove> = None;
        let mut heads = Vec::new();
        let mut part_jobs = Vec::new();
        for (job, found) in jobs.iter().zip(&chosen) {
            let Some(found) = found else { continue };
            let Some(mut dressed) = worn(job, found, models, &mut head, &mut issues) else {
                continue;
            };
            let model = without_covered(job, dressed.model, variant, &covering, &mut issues);
            let face = dressed.face.take();
            heads.extend(Found::of(parts.len(), &job.path, job.shape.as_ref(), face));
            if job.bespoke {
                heads.push(Found::bespoke(parts.len(), &job.path));
            }
            part_jobs.push(job);
            let materials = drawn_with(
                job,
                found,
                (&libraries, &unnamed),
                &mut reported,
                &mut issues,
            );
            parts.push(ArmourPart {
                name: job.part.clone(),
                origin: job.origin,
                file: job.path.clone(),
                model,
                materials,
                morphs: dressed.morphs,
                follows: dressed.follows,
                face: None,
            });
        }
        let skeleton = settled(
            body.skeleton,
            &mut parts,
            heads,
            &part_jobs,
            options.lod,
            &mut issues,
        );
        Ok(Self {
            variant,
            skeleton: Arc::new(skeleton),
            skeleton_file: body.skeleton_file,
            parts,
            issues,
            lod: options.lod,
        })
    }
}

/// Returns the body zones the worn clothing of the jobs that have a model covers.
fn covering<'j>(jobs: &'j [Job], chosen: &[Option<&Loaded>]) -> BTreeSet<&'j str> {
    covered(
        jobs.iter()
            .zip(chosen)
            .filter(|(_, found)| found.is_some())
            .flat_map(|(job, _)| job.zones.iter()),
    )
}

/// Returns the model of a job as it is worn: shaped by the job's DNA, if it has one, and following
/// the shaped head, if it follows it, with its morph targets; what follows the head has the
/// targets of the head moved to its own vertices. The first shaped head is remembered in `head`,
/// as it is shaped and with its morph targets.
fn worn(
    job: &Job,
    found: &Loaded,
    models: &dyn ModelProvider,
    head: &mut Option<HeadMove>,
    issues: &mut Vec<Issue>,
) -> Option<Worn> {
    let (model, morphs, face) = fitted(job, found, models, issues)?;
    if job.shape.is_some() && job.part == HEAD && head.is_none() {
        *head = Some(HeadMove {
            before: Arc::clone(&found.model),
            after: Arc::clone(&model),
            morphs: morphs.clone(),
        });
    }
    Some(match head {
        Some(moved) if job.follows_head => followed(&model, moved),
        _ => Worn {
            model,
            morphs,
            face,
            follows: Vec::new(),
        },
    })
}

/// Returns the materials a job is drawn with: its library, with the choices of its Character
/// applied. A library that cannot be read is listed once.
fn drawn_with(
    job: &Job,
    found: &Loaded,
    (libraries, unnamed): (&Libraries, &Arc<MaterialStatus>),
    reported: &mut HashSet<String>,
    issues: &mut Vec<Issue>,
) -> Arc<MaterialStatus> {
    let key = library_key(job, found);
    let library = key
        .as_ref()
        .and_then(|key| libraries.get(key))
        .map_or_else(|| Arc::clone(unnamed), Arc::clone);
    if let (Some(key), Some(reason)) = (key, library.problem().or_else(|| library.layer_problem()))
        && reported.insert(key)
    {
        issues.push(issue(job, IssueKind::MaterialLibrary { reason }));
    }
    job.look
        .as_ref()
        .and_then(|look| look.apply(&library))
        .map_or(library, Arc::new)
}

/// Returns the model of a job as it is worn, shaped by the job's DNA if it has one, and its morph
/// targets; a model that cannot be shaped is listed and left out, morph targets that cannot be
/// made are listed.
fn fitted(
    job: &Job,
    found: &Loaded,
    models: &dyn ModelProvider,
    issues: &mut Vec<Issue>,
) -> Option<(Arc<Model>, Vec<MorphTarget>, Option<Face>)> {
    let Some(dna) = &job.shape else {
        return Some((Arc::clone(&found.model), Vec::new(), None));
    };
    let head = job.part == HEAD;
    let shaped = match shaped(&found.model, &job.path, dna, &job.expressions, models, head) {
        Ok(shaped) => shaped,
        Err(reason) => {
            issues.push(issue(job, IssueKind::ShapeFailed { reason }));
            return None;
        }
    };
    let worn = shaped.into_parts();
    issues.extend(worn.kinds.into_iter().map(|kind| issue(job, kind)));
    let face = head.then(|| {
        let (facial, rig) = worn.face.unzip();
        Face {
            facial,
            rig: rig.unwrap_or_default(),
            numbers: worn.numbers,
        }
    });
    Some((Arc::new(worn.model), worn.targets, face))
}

fn provider_failure(path: &str, variant: BodyVariant, failure: ModelFailure) -> AssemblyError {
    match failure {
        ModelFailure::Fatal(source) => AssemblyError::Provider(source),
        other => AssemblyError::Body {
            variant: variant.code().to_owned(),
            path: path.to_owned(),
            reason: other.to_string(),
        },
    }
}

fn load_body(
    variant: BodyVariant,
    models: &dyn ModelProvider,
    lod: u32,
) -> Result<Body, AssemblyError> {
    let skeleton_file = variant.skeleton();
    let ProvidedModel { model, .. } = models
        .model(skeleton_file, lod)
        .map_err(|failure| provider_failure(skeleton_file, variant, failure))?;
    Ok(Body {
        skeleton: model,
        skeleton_file: skeleton_file.to_owned(),
    })
}

fn vertices(model: &Model) -> usize {
    model.lods.iter().map(Lod::vertex_count).max().unwrap_or(0)
}

/// Reads every distinct file the jobs name, a wave of files at a time in parallel, and stops
/// when the files read so far already hold `options.max_vertices` vertices.
fn read_models(
    jobs: &[Job],
    provider: &dyn ModelProvider,
    options: &AssemblyOptions,
) -> Result<Models, AssemblyError> {
    let mut seen = HashSet::new();
    let files: Vec<(String, &str)> = jobs
        .iter()
        .filter(|job| seen.insert(file_key(&job.path)))
        .map(|job| (file_key(&job.path), job.path.as_str()))
        .collect();
    let mut models = Models::new();
    let mut total = 0_usize;
    for wave in files.chunks(WAVE) {
        if total >= options.max_vertices {
            break;
        }
        let read: Vec<(String, Result<Loaded, ModelFailure>)> = wave
            .par_iter()
            .map(|(key, path)| {
                let result = provider.model(path, options.lod).map(|provided| Loaded {
                    entry: provided.entry,
                    model: Arc::new(provided.model),
                });
                (key.clone(), result)
            })
            .collect();
        for (key, result) in read {
            match result {
                Err(ModelFailure::Fatal(source)) => return Err(AssemblyError::Provider(source)),
                Ok(loaded) => {
                    total = total.saturating_add(vertices(&loaded.model));
                    models.insert(key, Ok(loaded));
                }
                Err(other) => {
                    models.insert(key, Err(other));
                }
            }
        }
    }
    Ok(models)
}

/// Picks the model of each job in order, within the vertex budget, and records why a job has none.
fn choose<'m>(
    jobs: &[Job],
    models: &'m Models,
    max_vertices: usize,
    issues: &mut Vec<Issue>,
) -> Vec<Option<&'m Loaded>> {
    let mut total = 0_usize;
    jobs.iter()
        .map(|job| {
            let path = job.path.clone();
            let kind = match models.get(&file_key(&job.path)) {
                Some(Ok(loaded)) => {
                    let count = vertices(&loaded.model);
                    if total.saturating_add(count) <= max_vertices {
                        total += count;
                        return Some(loaded);
                    }
                    IssueKind::TooManyVertices {
                        limit: max_vertices,
                    }
                }
                Some(Err(ModelFailure::Missing)) => IssueKind::GeometryMissing { path },
                Some(Err(other)) => IssueKind::GeometryUnreadable {
                    path,
                    reason: other.to_string(),
                },
                None => IssueKind::TooManyVertices {
                    limit: max_vertices,
                },
            };
            issues.push(issue(job, kind));
            None
        })
        .collect()
}

/// Returns the key under which the library of a job is cached: a bare name is relative to the
/// model's folder, any other name is the same for every model.
fn library_key(job: &Job, loaded: &Loaded) -> Option<String> {
    let name = library_name(job, loaded)?;
    let key = file_key(name);
    let key = key.strip_suffix(".mtl").unwrap_or(&key).to_owned();
    if key.contains('\\') {
        Some(key)
    } else {
        let folder = loaded
            .entry
            .rsplit_once(['\\', '/'])
            .map_or("", |(folder, _)| folder);
        Some(format!("{}\\{key}", folder.to_ascii_lowercase()))
    }
}

fn library_name<'a>(job: &'a Job, loaded: &'a Loaded) -> Option<&'a str> {
    job.material
        .as_deref()
        .or(loaded.model.material_library.as_deref())
}

/// Reads every distinct material library the jobs draw with once, in parallel.
fn read_libraries(
    jobs: &[Job],
    chosen: &[Option<&Loaded>],
    provider: &dyn ModelProvider,
) -> Libraries {
    let mut seen = HashSet::new();
    let wanted: Vec<(String, &str, &str)> = jobs
        .iter()
        .zip(chosen)
        .filter_map(|(job, loaded)| {
            let loaded = (*loaded)?;
            let name = library_name(job, loaded)?;
            let key = library_key(job, loaded)?;
            seen.insert(key.clone())
                .then_some((key, loaded.entry.as_str(), name))
        })
        .collect();
    wanted
        .par_iter()
        .map(|(key, entry, name)| (key.clone(), Arc::new(provider.library(entry, name))))
        .collect()
}
