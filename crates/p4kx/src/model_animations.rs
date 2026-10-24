// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::model::Model;
use p4kx_extraction::{AnimationSet, AnimationSummary};
use serde::Serialize;

use crate::query_table::one_line;

/// The JSON shape of an animation of `p4kx model glb`.
#[derive(Serialize)]
pub(crate) struct WrittenJson {
    name: String,
    duration_seconds: f32,
    additive: bool,
    channels: usize,
    keys: usize,
    skipped_tracks: Vec<u32>,
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
