use std::path::PathBuf;

use anyhow::Result;
use p4kx_extraction::model::{LodSelection, ModelError};
use p4kx_extraction::{
    AnimationChoice, AnimationSet, ConversionError, GltfError, ModelSource, PathIndex,
};

use crate::cli_model::{ModelAction, ModelArgs, ModelFileSource};
use crate::model_glb;
use crate::model_info::info;

pub(crate) type Opened = (ModelSource, AnimationSet);

/// Runs `p4kx model info` or `p4kx model glb`.
pub(crate) fn run(args: &ModelArgs, as_json: bool) -> Result<()> {
    match &args.action {
        ModelAction::Info(source) => info(
            &open(source, LodSelection::All, Some(&AnimationChoice::All))?,
            as_json,
        ),
        action @ ModelAction::Glb { .. } => model_glb::run(action, as_json),
    }
}

pub(crate) fn choice_of(animations: &[String]) -> Option<AnimationChoice> {
    if animations.is_empty() {
        None
    } else if animations
        .iter()
        .any(|name| name.eq_ignore_ascii_case("all"))
    {
        Some(AnimationChoice::All)
    } else {
        Some(AnimationChoice::Named(animations.to_vec()))
    }
}

/// Reads the animations `choice` selects for the model Entry `entry` of the Archive `index` is
/// over; none without a choice.
pub(crate) fn archive_animations(
    index: &PathIndex<'_>,
    entry: &str,
    source: &ModelFileSource,
    choice: Option<&AnimationChoice>,
) -> Result<AnimationSet> {
    Ok(choice
        .map(|choice| AnimationSet::from_archive(index, entry, choice, &source.animation_sources))
        .transpose()?
        .unwrap_or_default())
}

/// Reads the animations `choice` selects for the model file `source` names; none without a
/// choice.
pub(crate) fn disk_animations(
    source: &ModelFileSource,
    choice: Option<&AnimationChoice>,
) -> Result<AnimationSet> {
    let extra: Vec<PathBuf> = source.animation_sources.iter().map(PathBuf::from).collect();
    Ok(choice
        .map(|choice| AnimationSet::from_disk(&source.path, choice, &extra))
        .transpose()?
        .unwrap_or_default())
}

pub(crate) fn open(
    source: &ModelFileSource,
    lods: LodSelection,
    choice: Option<&AnimationChoice>,
) -> Result<Opened> {
    if let Some(entry) = &source.entry {
        let archive = crate::key::open(&source.path)?;
        let index = PathIndex::new(&archive);
        let model = ModelSource::from_index_with(&index, entry, lods)?;
        let animations = archive_animations(&index, entry, source, choice)?;
        return Ok((model, animations));
    }
    let model = ModelSource::from_file_with(&source.path, lods)?;
    let animations = disk_animations(source, choice)?;
    Ok((model, animations))
}

/// Returns whether `error` is a model failure caused by the command line: a geometry companion
/// that does not exist or a LOD the model does not have.
pub(crate) fn is_invalid_input(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ConversionError>(),
        Some(
            ConversionError::Model {
                source: ModelError::MissingGeometry { .. } | ModelError::NoSuchLod { .. },
                ..
            } | ConversionError::UnknownAnimations { .. }
                | ConversionError::NotAnimationFile { .. }
                | ConversionError::Glb {
                    source: GltfError::NoSuchLod { .. },
                    ..
                }
        )
    )
}

/// Returns whether `error` is a model failure caused by an unreadable or corrupt model or
/// companion.
pub(crate) fn is_archive_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ConversionError>(),
        Some(
            ConversionError::Model {
                source: ModelError::Companion { .. }
                    | ModelError::CompanionChunks { .. }
                    | ModelError::NoMesh { .. }
                    | ModelError::Corrupt { .. },
                ..
            } | ConversionError::Animation { .. }
        )
    )
}
