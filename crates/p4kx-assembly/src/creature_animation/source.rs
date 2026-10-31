// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{AnimationChoice, AnimationSet, ConversionError};

use super::database::{DatabaseProblem, clips};
use crate::cdf::{self, MAX_CDF_BYTES, is_character_definition};
use crate::creature_rig::{is_skinned, skeleton_of};
use crate::gear_clip::{ChrParams, MAX_CHRPARAMS_BYTES, params_path, read_params};
use crate::{ArchiveModels, Assembly, EntityNode, Issue, IssueKind, ModelFailure, ModelProvider};

/// The clips of a creature Entity, read from the Archive, and what kept some from being read.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct CreatureClips {
    /// The clips the Entity's animation database names that were found, as read by
    /// [`ArchiveModels::animations`].
    pub set: AnimationSet,
    /// What could not be found or read, as [`IssueKind::AnimationsUnavailable`].
    pub issues: Vec<Issue>,
}

/// Reads the clips the animation database of the creature `node` names, from the folders and the
/// track databases the parameter file of its skeleton lists.
///
/// A clip is found by its file name without extension. A name no folder has a clip for, such as a
/// blend space, is skipped. Nothing is read, and no issue is listed, when `assembly` writes no
/// skinned creature (see [`Assembly::write_glb_with`]) or `node` names no animation database.
///
/// # Errors
///
/// Returns the [`ConversionError`] of an animation file that exists and cannot be read.
pub fn creature_animations(
    models: &ArchiveModels<'_>,
    assembly: &Assembly,
    node: &EntityNode,
) -> Result<CreatureClips, ConversionError> {
    let (Some(database), Some(geometry)) = (
        node.animation_database(),
        assembly.root().and_then(|root| root.geometry()),
    ) else {
        return Ok(CreatureClips::default());
    };
    let skinned = assembly.parts.iter().any(|part| {
        part.model
            .as_deref()
            .is_some_and(|model| is_skinned(model, assembly.lod))
    });
    if !skinned || skeleton_of(&assembly.parts, &assembly.issues, assembly.lod).is_none() {
        return Ok(CreatureClips::default());
    }
    let mut reasons = Vec::new();
    let set = gather(models, (geometry, database), &mut reasons)?;
    let issues = reasons
        .into_iter()
        .map(|reason| Issue {
            path: String::new(),
            name: node.class_name().to_owned(),
            kind: IssueKind::AnimationsUnavailable { reason },
        })
        .collect();
    Ok(CreatureClips { set, issues })
}

/// Reads the clips of `database`, a path below `Data`, for the root geometry `geometry`, and adds
/// to `reasons` what kept some from being read; an empty set when nothing can be read at all.
fn gather(
    models: &ArchiveModels<'_>,
    (geometry, database): (&str, &str),
    reasons: &mut Vec<String>,
) -> Result<AnimationSet, ConversionError> {
    let params = match parameters(models, geometry) {
        Ok(params) => params,
        Err(reason) => {
            reasons.push(reason);
            return Ok(AnimationSet::default());
        }
    };
    if params.folders.is_empty() {
        reasons.push("the parameter file of the skeleton names no animation folder".to_owned());
        return Ok(AnimationSet::default());
    }
    let names = match clips(models, database) {
        Ok(read) => {
            reasons.extend(read.unread.into_iter().map(|(file, reason)| {
                format!(
                    "the animation database {file}, nested in {database}, cannot be read: {reason}"
                )
            }));
            read.names
        }
        Err(DatabaseProblem::Missing) => {
            reasons.push(format!(
                "the animation database {database} is not in the Archive"
            ));
            return Ok(AnimationSet::default());
        }
        Err(DatabaseProblem::Unreadable(reason)) => {
            reasons.push(format!(
                "the animation database {database} cannot be read: {reason}"
            ));
            return Ok(AnimationSet::default());
        }
    };
    let extra: Vec<String> = params
        .databases
        .iter()
        .filter_map(|path| models.entry_of(path))
        .collect();
    let folders: Vec<&str> = params.folders.iter().map(String::as_str).collect();
    let set = read_named(models, &folders, names, &extra)?;
    if set.animations.is_empty() {
        reasons.push(format!(
            "no clip of the animation database {database} is in the animation folders of the skeleton"
        ));
    }
    Ok(set)
}

/// Reads the parameter file beside the skeleton of the root geometry `geometry`: the file itself
/// for a skeleton, the one a character definition names otherwise.
fn parameters(models: &ArchiveModels<'_>, geometry: &str) -> Result<ChrParams, String> {
    let skeleton = if is_character_definition(geometry) {
        let bytes = models
            .file(geometry, MAX_CDF_BYTES)
            .map_err(|failure| format!("the character definition {geometry}: {failure}"))?;
        cdf::read(&bytes)
            .map_err(|reason| format!("the character definition {geometry}: {reason}"))?
            .skeleton
            .ok_or_else(|| format!("the character definition {geometry} names no skeleton"))?
    } else {
        geometry.to_owned()
    };
    let path = params_path(&skeleton);
    match models.file(&path, MAX_CHRPARAMS_BYTES) {
        Ok(bytes) => read_params(&bytes)
            .map_err(|reason| format!("the parameter file {path} cannot be read: {reason}")),
        Err(ModelFailure::Missing) => {
            Err(format!("the parameter file {path} is not in the Archive"))
        }
        Err(other) => Err(format!("the parameter file {path} cannot be read: {other}")),
    }
}

/// Reads the clips `names` below `folders` and the Entries `extra`; the names no clip has are left
/// out, which costs a second read only when there are some.
fn read_named(
    models: &ArchiveModels<'_>,
    folders: &[&str],
    names: Vec<String>,
    extra: &[String],
) -> Result<AnimationSet, ConversionError> {
    if names.is_empty() {
        return Ok(AnimationSet::default());
    }
    let choice = AnimationChoice::Named(names);
    match models.animations(folders, &choice, extra) {
        Err(ConversionError::UnknownAnimations { names: missing }) => {
            let AnimationChoice::Named(mut present) = choice else {
                return Ok(AnimationSet::default());
            };
            present.retain(|name| !missing.contains(name));
            if present.is_empty() {
                return Ok(AnimationSet::default());
            }
            models.animations(folders, &AnimationChoice::Named(present), extra)
        }
        other => other,
    }
}
