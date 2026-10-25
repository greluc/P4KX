// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use anyhow::Result;
use p4kx_assembly::{ArchiveModels, BodyVariant};
use p4kx_extraction::model::Model;
use p4kx_extraction::{AnimationSet, AnimationSummary, ConversionError};
use serde::Serialize;

use crate::model::choice_of;
use crate::query_table::one_line;

/// The folders `--anim` looks in before the folders of the body, and whether it falls back to
/// those.
#[derive(Debug, Default)]
pub(crate) struct AnimationFolders {
    folders: Vec<String>,
    fallback: bool,
}

impl AnimationFolders {
    /// Looks only in `folders`, the folders `--anim-folder` names; none means the folders of the
    /// body.
    pub(crate) fn only(folders: &[String]) -> Self {
        Self {
            folders: folders.to_vec(),
            fallback: false,
        }
    }

    /// Looks in the folder of the story character's own animations first, `Animations/pu/<body>/
    /// <name>`, where `name` is the name of the head's own folder `folder` up to its first
    /// underscore; a name or path that folder lacks is looked for in the folders of the body.
    /// Only the folders of the body when the Archive has no such folder.
    pub(crate) fn of_npc(models: &ArchiveModels<'_>, variant: BodyVariant, folder: &str) -> Self {
        let name = folder
            .rsplit(['/', '\\'])
            .next()
            .and_then(|last| last.split('_').next())
            .filter(|name| !name.is_empty());
        let own = name
            .map(|name| format!("Animations/pu/{}/{name}", variant.code()))
            .filter(|own| models.has_folder(own));
        Self {
            folders: own.into_iter().collect(),
            fallback: true,
        }
    }

    /// Returns the folder looked in first, if there is one and no `--anim-folder` was given.
    pub(crate) fn own(&self) -> Option<&str> {
        self.folders
            .first()
            .map(String::as_str)
            .filter(|_| self.fallback)
    }
}

/// Reads the animations the options `--anim` and `--anim-from` name for a Character of `variant`
/// from the folders `folders` choose, else those of its body and head; none without `--anim`.
pub(crate) fn character_animations(
    models: &ArchiveModels<'_>,
    variant: BodyVariant,
    (names, sources): (&[String], &[String]),
    folders: &AnimationFolders,
) -> Result<AnimationSet> {
    let Some(choice) = choice_of(names) else {
        return Ok(AnimationSet::default());
    };
    if !folders.folders.is_empty() {
        let own: Vec<&str> = folders.folders.iter().map(String::as_str).collect();
        match models.animations(&own, &choice, sources) {
            Ok(found) => return Ok(found),
            Err(ConversionError::UnknownAnimations { .. }) if folders.fallback => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(models.animations(&variant.animation_folders(), &choice, sources)?)
}

/// The JSON shape of an animation of `p4kx model glb`.
#[derive(Serialize)]
pub(crate) struct WrittenJson {
    name: String,
    duration_seconds: f32,
    additive: bool,
    channels: usize,
    keys: usize,
    skipped_tracks: Vec<u32>,
    #[serde(skip_serializing_if = "is_zero")]
    skipped_weights: usize,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
const fn is_zero(value: &usize) -> bool {
    *value == 0
}

impl WrittenJson {
    pub(crate) fn of(summary: &AnimationSummary) -> Self {
        Self {
            name: one_line(&summary.name),
            duration_seconds: summary.duration_seconds,
            additive: summary.additive,
            channels: summary.channels,
            keys: summary.keys,
            skipped_tracks: summary.skipped_tracks.clone(),
            skipped_weights: summary.skipped_weights,
        }
    }
}

/// Writes the table of the animations of `set` after a blank line; nothing without any.
pub(crate) fn write_table(
    out: &mut impl Write,
    model: &Model,
    set: &AnimationSet,
) -> io::Result<()> {
    if set.animations.is_empty() {
        return Ok(());
    }
    writeln!(out)?;
    writeln!(
        out,
        "{:<40}  {:>8}  {:>6}  {:>3}  {:>6}  {:>5}  {:>8}  {:<8}  Source",
        "Animation", "Seconds", "Frames", "FPS", "Tracks", "Bound", "Keys", "Additive"
    )?;
    for (animation, origin) in set.animations.iter().zip(&set.origins) {
        writeln!(
            out,
            "{:<40}  {:>8.3}  {:>6}  {:>3}  {:>6}  {:>5}  {:>8}  {:<8}  {}",
            one_line(&animation.name),
            animation.duration_seconds(),
            animation.end_frame,
            animation.frames_per_second,
            animation.tracks.len(),
            animation.bind(&model.bones).bound.len(),
            animation.key_count(),
            if animation.additive { "yes" } else { "no" },
            one_line(origin)
        )?;
    }
    Ok(())
}

/// Returns the line `Animations:` of `p4kx model info`.
pub(crate) fn count_line(set: &AnimationSet) -> String {
    let unread = if set.unread.is_empty() {
        String::new()
    } else {
        format!(", {} skipped", set.unread.len())
    };
    format!(
        "{} (from {} files{unread})",
        set.animations.len(),
        set.files
    )
}

/// Writes what could not be read, one line each.
pub(crate) fn write_unread(out: &mut impl Write, set: &AnimationSet) -> io::Result<()> {
    for unread in &set.unread {
        let name = unread
            .name
            .as_deref()
            .map_or_else(String::new, |name| format!(" animation {}", one_line(name)));
        writeln!(
            out,
            "skipped{name} of {}: {}",
            one_line(&unread.source),
            one_line(&unread.reason)
        )?;
    }
    Ok(())
}

/// Writes the notes of a GLB export about its animations.
pub(crate) fn write_notes(
    out: &mut impl Write,
    summaries: &[AnimationSummary],
    set: &AnimationSet,
) -> io::Result<()> {
    if summaries.is_empty() {
        return Ok(());
    }
    let channels: usize = summaries.iter().map(|summary| summary.channels).sum();
    let written = summaries
        .iter()
        .filter(|summary| summary.channels > 0)
        .count();
    writeln!(out, "{written} animations, {channels} channels")?;
    for summary in summaries {
        let name = one_line(&summary.name);
        if summary.channels == 0 {
            writeln!(out, "left out animation {name}: no track matches a bone")?;
        } else if !summary.skipped_tracks.is_empty() {
            writeln!(
                out,
                "animation {name}: left out {} tracks whose bone the skeleton lacks: {}",
                summary.skipped_tracks.len(),
                list_ids(&summary.skipped_tracks)
            )?;
        }
        if summary.skipped_weights > 0 {
            writeln!(
                out,
                "animation {name}: left out {} runs of morph weights that do not fit the targets of the mesh",
                summary.skipped_weights
            )?;
        }
        if summary.additive {
            writeln!(
                out,
                "animation {name} is additive: its keys are offsets to a pose, which glTF cannot say, so it needs a base animation"
            )?;
        }
    }
    write_unread(out, set)
}

fn list_ids(ids: &[u32]) -> String {
    const SHOWN: usize = 8;
    let mut text: Vec<String> = ids
        .iter()
        .take(SHOWN)
        .map(|id| format!("{id:#010x}"))
        .collect();
    if ids.len() > SHOWN {
        text.push(format!("and {} more", ids.len() - SHOWN));
    }
    text.join(", ")
}
