// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::{
    AnimationInfo, AnimationListing, AnimationSet, AnimationSummary, UnreadAnimation,
};
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

/// Writes the table of `animations` after a blank line; nothing without any.
pub(crate) fn write_table(out: &mut impl Write, animations: &[AnimationInfo]) -> io::Result<()> {
    if animations.is_empty() {
        return Ok(());
    }
    writeln!(out)?;
    writeln!(
        out,
        "{:<40}  {:>8}  {:>6}  {:>3}  {:>6}  {:>5}  {:>8}  {:<8}  Source",
        "Animation", "Seconds", "Frames", "FPS", "Tracks", "Bound", "Keys", "Additive"
    )?;
    for animation in animations {
        writeln!(
            out,
            "{:<40}  {:>8.3}  {:>6}  {:>3}  {:>6}  {:>5}  {:>8}  {:<8}  {}",
            animation.name,
            animation.duration_seconds,
            animation.end_frame,
            animation.frames_per_second,
            animation.tracks,
            animation.bound_tracks,
            animation.keys,
            if animation.additive { "yes" } else { "no" },
            animation.source
        )?;
    }
    Ok(())
}

/// Returns the line `Animations:` of `p4kx model info`.
pub(crate) fn count_line(listing: &AnimationListing) -> String {
    let unread = if listing.unread.is_empty() {
        String::new()
    } else {
        format!(", {} skipped", listing.unread.len())
    };
    format!(
        "{} (from {} files{unread})",
        listing.animations.len(),
        listing.files
    )
}

/// How many names a summarised `skipped` line lists.
const UNREAD_EXAMPLES: usize = 3;

/// Writes what could not be read: one line for each reason that holds a single animation or
/// file, and one line with the count and a few names for each reason that holds several.
pub(crate) fn write_unread(out: &mut impl Write, unread: &[UnreadAnimation]) -> io::Result<()> {
    let mut groups: Vec<(bool, &str, Vec<&UnreadAnimation>)> = Vec::new();
    for unread in unread {
        let named = unread.name.is_some();
        match groups
            .iter_mut()
            .find(|(kind, reason, _)| *kind == named && *reason == unread.reason)
        {
            Some((_, _, members)) => members.push(unread),
            None => groups.push((named, &unread.reason, vec![unread])),
        }
    }
    for (named, reason, members) in groups {
        if let [unread] = members.as_slice() {
            let name = unread
                .name
                .as_deref()
                .map_or_else(String::new, |name| format!(" animation {}", one_line(name)));
            writeln!(
                out,
                "skipped{name} of {}: {}",
                one_line(&unread.source),
                one_line(reason)
            )?;
            continue;
        }
        let what = if named { "animations" } else { "files" };
        let examples: Vec<String> = members
            .iter()
            .take(UNREAD_EXAMPLES)
            .map(|unread| one_line(unread.name.as_deref().unwrap_or(&unread.source)))
            .collect();
        writeln!(
            out,
            "skipped {} {what}: {} (e.g. {})",
            members.len(),
            one_line(reason),
            examples.join(", ")
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
    write_unread(out, &set.unread)
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
